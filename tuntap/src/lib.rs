#![recursion_limit = "128"]

#[cfg(target_os = "linux")]
#[macro_use]
extern crate bitflags;
extern crate bytes;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate ifcontrol;
#[cfg(unix)]
extern crate ifstructs;
#[cfg(windows)]
extern crate ipconfig;
#[cfg(unix)]
extern crate libc;
extern crate mio;
#[cfg(windows)]
extern crate miow;
#[cfg(unix)]
#[macro_use]
extern crate nix;
extern crate tokio;
#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate winreg;

use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::string::ToString;
use std::sync::{Arc, Mutex, Weak};
use std::thread;

use bytes::{Bytes, BytesMut};
use futures::sync::mpsc;
use futures::{Future, Sink, Stream};

pub use impls::*;

mod impls;

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
        }
        .to_string()
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
    #[cfg(not(windows))]
    inner: File,
    #[cfg(windows)]
    inner: impls::Handle,
    #[cfg(windows)]
    read_overlapped: Arc<miow::Overlapped>,
    #[cfg(windows)]
    write_overlapped: Arc<miow::Overlapped>,
    #[allow(dead_code)]
    info: Arc<Mutex<VirtualInterfaceInfo>>,
    _closer: ::std::marker::PhantomData<C>,
}

impl<C> Descriptor<C>
where
    C: DescriptorCloser,
{
    fn from_file(file: File, info: &Arc<Mutex<VirtualInterfaceInfo>>) -> Descriptor<C> {
        //TODO: return io::Result
        #[cfg(windows)]
        use std::os::windows::io::IntoRawHandle;

        #[cfg(windows)]
        let inner = {
            let inner = Handle::new(file.into_raw_handle());
            inner.set_no_timeouts().expect("Could not unset timeout");
            inner
        };

        Descriptor {
            #[cfg(not(windows))]
            inner: file,
            #[cfg(windows)]
            inner,
            #[cfg(windows)]
            read_overlapped: Arc::new(miow::Overlapped::initialize_with_autoreset_event().unwrap()),
            #[cfg(windows)]
            write_overlapped: Arc::new(miow::Overlapped::initialize_with_autoreset_event().unwrap()),
            _closer: Default::default(),
            info: info.clone(),
        }
    }

    fn try_clone(&self) -> Result<Self, TunTapError> {
        let cloned = self.inner.try_clone()?;

        #[cfg(windows)]
        cloned.set_no_timeouts();

        Ok(Descriptor {
            inner: cloned,
            #[cfg(windows)]
            read_overlapped: self.read_overlapped.clone(),
            #[cfg(windows)]
            write_overlapped: self.write_overlapped.clone(),
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

#[cfg(windows)]
impl<C> Read for Descriptor<C>
where
    C: DescriptorCloser,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe { self.inner.read_overlapped_wait(buf, self.read_overlapped.raw()) }
    }
}

#[cfg(not(windows))]
impl<C> Read for Descriptor<C>
where
    C: DescriptorCloser,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(windows)]
impl<C> Write for Descriptor<C>
where
    C: DescriptorCloser,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe { self.inner.write_overlapped_wait(buf, self.write_overlapped.raw()) }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(not(windows))]
impl<C> Write for Descriptor<C>
where
    C: DescriptorCloser,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
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

    pub fn pop_split_channels(&mut self) -> Option<(impl Sink<SinkItem = Bytes, SinkError = io::Error>, impl Stream<Item = BytesMut, Error = io::Error>)> {
        //TODO: share handle through Arc, instead of clone?
        let mut write_file = self.pop_file()?;
        let mut read_file = write_file.try_clone().unwrap();

        //hardcoded buffer size. move to builder somehow?
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<BytesMut>(4);
        let (incoming_tx, incoming_rx) = mpsc::channel::<Bytes>(4);

        let _handle_outgoing = thread::spawn(move || {
            let mut buf = BytesMut::from(vec![0u8; ::RESERVE_AT_ONCE]);
            loop {
                match read_file.read(&mut buf) {
                    Ok(len) => {
                        let packet = buf.split_to(len);
                        let cur_capacity = buf.len();
                        if cur_capacity < ::MTU {
                            buf.resize(::RESERVE_AT_ONCE, 0);
                        }
                        if let Err(_e) = outgoing_tx.clone().send(packet).wait() {
                            //stop thread because other side is gone
                            break;
                        }
                    },
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        eprintln!("TimedOut on outlet read. ignoring");
                        // do nothing
                    },
                    Err(ref e) => {
                        eprintln!("Error {:?} on outlet. Stop read thread", e);
                        break;
                    }
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
                write_file.write_all(&mut packet).expect("Error writing to outlet. Exiting thread");
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
