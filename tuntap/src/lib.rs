#![recursion_limit = "128"]

#[cfg(target_os = "linux")]
#[macro_use]
extern crate bitflags;

use std::fs::File;
use std::io;
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::io::{FromRawHandle, IntoRawHandle};
use std::string::ToString;
use std::sync::{Arc, Weak};
use std::thread;

use bytes::{Bytes, BytesMut};
use futures::{pin_mut, select};
use futures::channel::mpsc;
use futures::executor::block_on;
use futures::prelude::*;
use parking_lot::Mutex;
use thiserror::Error;

//pub use evented::EventedDescriptor;
pub use impls::*;
use stop_handle::stop_handle;

mod evented;
mod impls;
//mod poll_evented;

//pub use poll_evented::PollEvented;

const MTU: usize = 65536;
const RESERVE_AT_ONCE: usize = 128 * MTU; //reserve large buffer once

#[derive(Debug, Error)]
pub enum TunTapError {
    #[cfg(unix)]
    #[error("nix error: `{0}`")]
    Nix(#[from] ::nix::Error),
    #[error("io error: `{0}`")]
    Io(#[from] ::std::io::Error),
    #[error("ifcontrol error: `{0}`")]
    IfControl(#[from] ifcontrol::IfError),
    #[error("not found: `{msg}`")]
    NotFound { msg: String },
    #[error("max number {max} of virtual interfaces reached")]
    MaxNumberReached { max: usize },
    #[error("name too long {s}, max {max}")]
    NameTooLong { s: usize, max: usize },
    #[error("bad arguments: {msg}")]
    BadArguments { msg: String },
    #[error("backend is not supported: {msg}")]
    NotSupported { msg: String },
    #[error("driver not found: {msg}")]
    DriverNotFound { msg: String },
    #[error("bad data received: {msg}")]
    BadData { msg: String },
    #[error("device busy")]
    Busy,
    #[error("error: {msg}")]
    Other { msg: String },
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

pub trait DescriptorCloser: Unpin
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
    inner: File,
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
            inner: file,
            _closer: Default::default(),
            info: info.clone(),
        }
    }

    fn try_clone(&self) -> Result<Self, TunTapError> {
        Ok(Descriptor {
            inner: self.inner.try_clone()?,
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

#[cfg(not(windows))]
impl<C> Read for Descriptor<C>
    where
        C: DescriptorCloser,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(not(windows))]
impl<C> Write for Descriptor<C>
    where
        C: DescriptorCloser,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    #[inline]
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
        C: crate::DescriptorCloser,
{
    pub fn pop_file(&mut self) -> Option<Descriptor<C>> {
        self.queues.pop()
    }

    #[cfg(not(windows))]
    pub fn pop_split_channels(
        &mut self,
    ) -> Option<(
        impl Sink<Bytes, Error=io::Error>,
        impl Stream<Item=BytesMut>,
    )> {
        let mut write_file = self.pop_file()?;
        let mut read_file = write_file.try_clone().unwrap();

        let (stop_writer_tx, stop_writer_rx) = stop_handle();

        //hardcoded buffer size. move to builder somehow?
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<BytesMut>(4);
        let (incoming_tx, mut incoming_rx) = mpsc::channel::<Bytes>(4);

        let _handle_outgoing = thread::spawn(move || {
            let mut buf = BytesMut::with_capacity(crate::RESERVE_AT_ONCE);
            buf.resize(crate::RESERVE_AT_ONCE, 0);

            loop {
                match read_file.read(&mut buf) {
                    Ok(len) => {
                        if len > 0 {
                            let packet = buf.split_to(len);
                            let cur_capacity = buf.len();
                            if cur_capacity < crate::MTU {
                                buf.resize(crate::RESERVE_AT_ONCE, 0);
                            }
                            if let Err(_e) = block_on(outgoing_tx.clone().send(packet)) {
                                //stop thread because other side is gone
                                break;
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        // do nothing
                    }
                    Err(ref _e) => {
                        break;
                    }
                }
            }
            let _ = stop_writer_tx.stop(());
        });

        let _handle_incoming = thread::spawn(move || {
            let forward = async move {
                while let Some(mut packet) = incoming_rx.next().await {
                    write_file.write_all(&mut packet)?;
                };

                Ok::<(), io::Error>(())
            }.fuse();


            let perform = async move {
                pin_mut!(forward);
                pin_mut!(stop_writer_rx);

                select! {
                    _ = forward => {},
                    _ = stop_writer_rx => {},
                }
            };

            block_on(perform);
        });

        Some((
            incoming_tx.sink_map_err(|_| io::Error::new(io::ErrorKind::Other, "mpsc error")),
            outgoing_rx,
        ))
    }
}

impl<D> Virtualnterface<D> {
    pub fn info(&self) -> Option<VirtualInterfaceInfo> {
        self.info.upgrade().map(|l| (*l.lock()).clone())
    }
}
