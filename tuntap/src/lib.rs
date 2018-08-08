#![recursion_limit="128"]

extern crate bytes;
extern crate futures;
extern crate mio;
extern crate tokio;
#[macro_use]
extern crate error_chain;

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

mod errors;
mod impls;

pub use impls::*;

use errors::Result;

use futures::sync::mpsc;
use futures::{Future, Sink, Stream};
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::string::ToString;
use std::sync::{Arc, Mutex, Weak};
use std::thread;

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
    fn close_descriptor(d: &mut Descriptor<Self>) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct VirtualInterfaceInfo {
    name: String,
    iface_type: VirtualInterfaceType,
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

    fn try_clone(&self) -> Result<Self> {
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

    pub fn pop_channels_spawn_threads(
        &mut self,
        buffer: usize,
    ) -> Option<Result<(mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>)>> {
        let mut write_file = self.pop_file()?;
        let mut read_file = match write_file.try_clone() {
            Ok(f) => f,
            Err(e) => return Some(Err(e.into())),
        };

        let (outgoing_tx, outgoing_rx) = mpsc::channel::<Vec<u8>>(buffer);
        let (incoming_tx, incoming_rx) = mpsc::channel::<Vec<u8>>(buffer);

        let _handle_outgoing = thread::spawn(move || loop {
            let mut v = vec![0u8; 2000];
            let len = read_file.read(&mut v).unwrap();
            v.resize(len, 0);
            if let Err(_e) = outgoing_tx.clone().send(v).wait() {
                //stop thread because other side is gone
                break;
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

        Some(Ok((incoming_tx, outgoing_rx)))
    }
}

impl<D> Virtualnterface<D> {
    pub fn info(&self) -> Option<VirtualInterfaceInfo> {
        self.info.upgrade().map(|l| (*l.lock().unwrap()).clone())
    }
}
