#![recursion_limit = "128"]

#[cfg(any(target_os = "linux", target_os = "android"))]
#[macro_use]
extern crate bitflags;
extern crate bytes;
extern crate futures;
extern crate mio;
extern crate tokio;
#[macro_use]
extern crate failure;

#[cfg(unix)]
extern crate ifstructs;

extern crate ifcontrol;

#[cfg(unix)]
extern crate libc;
#[cfg(unix)]
#[macro_use]
extern crate nix;

#[cfg(windows)]
extern crate ipconfig;
#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate winreg;

mod impls;

pub use impls::*;

use bytes::{Bytes, BytesMut};
use futures::sync::mpsc;
use futures::{Future, Sink, Stream};
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::string::ToString;
use std::sync::{Arc, Mutex, Weak};
use std::thread;

const MTU: usize = 2000;
const RESERVE_AT_ONCE: usize = 65536; //reserve large buffer once

#[derive(Debug, Fail)]
#[fail(display = "tuntap error")]
pub enum TunTapError {
    #[cfg(unix)]
    #[fail(display = "nix error: {}", _0)]
    Nix(#[cause] ::nix::Error),
    #[fail(display = "io error: {}", _0)]
    Io(#[cause] ::std::io::Error),
    #[fail(display = "ifcontrol error: {}", _0)]
    IfControl(#[cause] ifcontrol::IfError),
    #[fail(display = "not found: {}", msg)]
    NotFound { msg: String },
    #[fail(display = "max number {} of virtual interfaces reached", max)]
    MaxNumberReached { max: usize },
    #[fail(display = "name too long {}, max {}", s, max)]
    NameTooLong { s: usize, max: usize },
    #[fail(display = "bad arguments: {}", msg)]
    BadArguments { msg: String },
    #[fail(display = "backend is not supported: {}", msg)]
    NotSupported { msg: String },
    #[fail(display = "driver not found: {}", msg)]
    DriverNotFound { msg: String },
    #[fail(display = "bad data received: {}", msg)]
    BadData { msg: String },
    #[fail(display = "device busy")]
    Busy,
    #[fail(display = "error: {}", msg)]
    Other { msg: String },
}

#[cfg(unix)]
impl From<::nix::Error> for TunTapError {
    fn from(e: ::nix::Error) -> TunTapError {
        TunTapError::Nix(e)
    }
}

impl From<::std::io::Error> for TunTapError {
    fn from(e: ::std::io::Error) -> TunTapError {
        TunTapError::Io(e)
    }
}

impl From<ifcontrol::IfError> for TunTapError {
    fn from(e: ifcontrol::IfError) -> TunTapError {
        TunTapError::IfControl(e)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VirtualInterfaceType {
    Tun,
    Tap,
}

impl ToString for VirtualInterfaceType {
    fn to_string(&self) -> String {
        match *self {
            VirtualInterfaceType::Tap => "tap",
            VirtualInterfaceType::Tun => "tun",
        }.to_string()
    }
}

pub trait DescriptorCloser
where
    Self: std::marker::Sized + Send + 'static,
{
    fn close_descriptor(d: &mut Descriptor<Self>) -> Result<(), TunTapError>;
}

#[derive(Clone, Debug)]
pub struct VirtualInterfaceInfo {
    pub name: String,
    pub iface_type: VirtualInterfaceType,
}

pub struct Descriptor<C: DescriptorCloser> {
    file: File,
    #[allow(dead_code)]
    info: Arc<Mutex<VirtualInterfaceInfo>>,
    _closer: ::std::marker::PhantomData<C>,
}

impl<C> Descriptor<C>
where
    C: DescriptorCloser,
{
    fn from_file(file: File, info: &Arc<Mutex<VirtualInterfaceInfo>>) -> Descriptor<C> {
        Descriptor {
            file,
            _closer: Default::default(),
            info: info.clone(),
        }
    }

    fn try_clone(&self) -> Result<Self, TunTapError> {
        Ok(Descriptor {
            file: self.file.try_clone()?,
            _closer: Default::default(),
            info: self.info.clone(),
        })
    }
}

impl<C> Drop for Descriptor<C>
where
    C: DescriptorCloser,
{
    fn drop(&mut self) {
        C::close_descriptor(self).unwrap()
    }
}

impl<C> Read for Descriptor<C>
where
    C: DescriptorCloser,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl<C> Write for Descriptor<C>
where
    C: DescriptorCloser,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

pub struct Virtualnterface<D> {
    queues: Vec<D>,
    info: Weak<Mutex<VirtualInterfaceInfo>>,
}

impl<C> Virtualnterface<Descriptor<C>>
where
    C: ::DescriptorCloser,
{
    pub fn pop_file(&mut self) -> Option<Descriptor<C>> {
        self.queues.pop()
    }

    pub fn pop_split_channels(
        &mut self,
    ) -> Option<(
        impl Sink<SinkItem = Bytes, SinkError = io::Error>,
        impl Stream<Item = Bytes, Error = io::Error>,
    )> {
        let mut write_file = self.pop_file()?;
        let mut read_file = write_file.try_clone().unwrap();

        //hardcoded buffer size. move to builder somehow?
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<Bytes>(4);
        let (incoming_tx, incoming_rx) = mpsc::channel::<Bytes>(4);

        let _handle_outgoing = thread::spawn(move || {
            let mut buf = BytesMut::from(vec![0u8; ::RESERVE_AT_ONCE]);
            loop {
                let len = read_file.read(&mut buf).expect("Couldn't read from virtual interface");
                let packet = buf.split_to(len);
                let cur_capacity = buf.len();
                if cur_capacity < ::MTU {
                    buf.resize(::RESERVE_AT_ONCE, 0);
                }
                if let Err(_e) = outgoing_tx.clone().send(packet.freeze()).wait() {
                    //stop thread because other side is gone
                    break;
                }
            }
        });

        let _handle_incoming = thread::spawn(move || {
            for input in incoming_rx.wait() {
                if let Err(()) = input {
                    //stop thread because other side is gone
                    break;
                }
                let mut packet = input.unwrap();
                write_file.write_all(&mut packet).unwrap();
            }
        });

        Some((
            Box::new(incoming_tx.sink_map_err(|_| io::Error::new(io::ErrorKind::Other, "mpsc error"))),
            Box::new(outgoing_rx.map_err(|_| io::Error::new(io::ErrorKind::Other, "mpsc error"))),
        ))
    }
}

impl<D> Virtualnterface<D> {
    pub fn info(&self) -> Option<VirtualInterfaceInfo> {
        self.info.upgrade().map(|l| (*l.lock().unwrap()).clone())
    }
}
