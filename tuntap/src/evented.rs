#[cfg(windows)]
use impls::set_iface_status;
#[cfg(windows)]
use parking_lot::Mutex;
use std::io;
use std::io::{Read, Write};
#[cfg(windows)]
use std::marker::PhantomData;
#[cfg(unix)]
use std::os::unix::prelude::*;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::sync::Arc;

use bytes::{Bytes, BytesMut, IntoBuf};
use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};
use mio;
use mio::event::Evented;
#[cfg(unix)]
use mio::unix::EventedFd;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::reactor::PollEvented2;

#[cfg(windows)]
use impls::async::AsyncFile;

#[cfg(unix)]
impl<C> Evented for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn register(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt) -> io::Result<()> {
        EventedFd(&self.io().as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt) -> io::Result<()> {
        EventedFd(&self.io().as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.io().as_raw_fd()).deregister(poll)
    }
}

#[cfg(windows)]
impl<C> Evented for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn register(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt) -> io::Result<()> {
        self.inner.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt) -> io::Result<()> {
        self.inner.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        self.inner.deregister(poll)
    }
}

#[cfg(unix)]
pub struct EventedDescriptor<C: ::DescriptorCloser>(::Descriptor<C>);

#[cfg(windows)]
pub struct EventedDescriptor<C: ::DescriptorCloser> {
    inner: AsyncFile,
    _closer: PhantomData<C>,
    info: Arc<Mutex<::VirtualInterfaceInfo>>,
}

#[cfg(windows)]
impl<C> Drop for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn drop(&mut self) {
        use std::process::Command;

        eprintln!("Bring iface down");
        let _ = set_iface_status(self.inner.as_raw_handle(), false);
        let iface_name = self.info.lock().name.clone();

        let mut disable = Command::new("netsh");
        disable.arg("interface").arg("set").arg("interface").arg(&iface_name).arg("admin=disable");
        eprintln!("Executing command {:?}", disable);
        eprintln!("Result: {:?}", disable.output());
        disable.output().expect("Could not re-enable TAP iface: disable failed");
        let mut enable = Command::new("netsh");
        enable.arg("interface").arg("set").arg("interface").arg(&iface_name).arg("admin=enable");
        eprintln!("Executing command {:?}", enable);
        eprintln!("Result: {:?}", enable.output());
    }
}

impl<C> EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    #[cfg(unix)]
    fn io_mut(&mut self) -> &mut ::Descriptor<C> {
        &mut self.0
    }

    #[cfg(unix)]
    fn io(&self) -> &::Descriptor<C> {
        &self.0
    }

    #[cfg(windows)]
    fn io_mut(&mut self) -> &mut AsyncFile {
        &mut self.inner
    }
}

#[cfg(unix)]
impl<C> From<::Descriptor<C>> for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn from(d: ::Descriptor<C>) -> EventedDescriptor<C> {
        EventedDescriptor(d)
    }
}

#[cfg(windows)]
impl<C> From<::Descriptor<C>> for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn from(d: ::Descriptor<C>) -> EventedDescriptor<C> {
        EventedDescriptor {
            inner: d.inner.try_clone().expect("Could not clone handle").into(),
            info: d.info.clone(),
            _closer: d._closer,
        }
    }
}

impl<C> Read for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.io_mut().read(buf)
    }
}

impl<C> Write for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.io_mut().flush()
    }
}

impl<C> AsyncRead for EventedDescriptor<C> where C: ::DescriptorCloser {}

impl<C> AsyncWrite for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        unimplemented!()
    }
}

pub struct AsyncDescriptor<C>
where
    C: ::DescriptorCloser,
{
    inner: PollEvented2<EventedDescriptor<C>>,
    incoming: Option<io::Cursor<Bytes>>,
    outgoing: BytesMut,
}

impl<C> From<PollEvented2<EventedDescriptor<C>>> for AsyncDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn from(f: PollEvented2<EventedDescriptor<C>>) -> AsyncDescriptor<C> {
        AsyncDescriptor {
            inner: f,
            incoming: None,
            outgoing: BytesMut::with_capacity(::RESERVE_AT_ONCE),
        }
    }
}

impl<C> Stream for AsyncDescriptor<C>
where
    C: ::DescriptorCloser,
{
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.inner.read_buf(&mut self.outgoing).and_then(|res| {
            if let Async::Ready(n) = res {
                if n > 0 {
                    let packet = self.outgoing.split_to(n);
                    let cur_capacity = self.outgoing.capacity();
                    if cur_capacity < ::MTU {
                        self.outgoing.reserve(::RESERVE_AT_ONCE);
                    }
                    return Ok(Async::Ready(Some(packet)));
                }
            }
            Ok(Async::NotReady)
        })
    }
}

impl<C> Sink for AsyncDescriptor<C>
where
    C: ::DescriptorCloser,
{
    type SinkItem = Bytes;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, io::Error> {
        if self.incoming.is_some() {
            self.poll_complete()?;
            if self.incoming.is_some() {
                return Ok(AsyncSink::NotReady(item));
            }
        }
        self.incoming = Some(item.into_buf());
        self.poll_complete()?;
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        let res = if let Some(ref mut buf) = self.incoming {
            self.inner.write_buf(buf).and_then(move |res| {
                if let Async::Ready(n) = res {
                    if n == 0 {
                        Ok(Async::NotReady)
                    } else if n == buf.get_ref().len() {
                        Ok(Async::Ready(()))
                    } else {
                        Err(io::Error::new(io::ErrorKind::Other, "Failed to send whole datagram"))
                    }
                } else {
                    Ok(Async::NotReady)
                }
            })
        } else {
            Ok(Async::Ready(()))
        };
        if let Ok(Async::Ready(_)) = res {
            self.incoming = None;
        };
        res
    }

    fn close(&mut self) -> Poll<(), io::Error> {
        Ok(().into())
    }
}

impl<C> ::Virtualnterface<PollEvented2<EventedDescriptor<C>>>
where
    C: ::DescriptorCloser,
{
    pub fn pop_split_channels(&mut self) -> Option<(impl Sink<SinkItem = Bytes, SinkError = io::Error>, impl Stream<Item = BytesMut, Error = io::Error>)> {
        if let Some(q) = self.queues.pop() {
            Some(AsyncDescriptor::from(q).split())
        } else {
            None
        }
    }
}
