#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux_common;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "android")]
pub use self::android::*;
#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;
#[cfg(target_os = "linux")]
pub use self::linux::*;
#[cfg(target_os = "macos")]
pub use self::macos::*;

use bytes::{Bytes, BytesMut, IntoBuf};
use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};
use mio;
use mio::event::Evented;
use mio::unix::EventedFd;
use std::io;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::reactor::PollEvented2;

impl<C> AsRawFd for ::Descriptor<C>
where
    C: ::DescriptorCloser,
{
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl<C> Evented for super::EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn register(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.0.as_raw_fd()).deregister(poll)
    }
}

pub struct EventedDescriptor<C: ::DescriptorCloser>(::Descriptor<C>);

impl<C> From<::Descriptor<C>> for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn from(d: ::Descriptor<C>) -> EventedDescriptor<C> {
        EventedDescriptor(d)
    }
}

impl<C> Read for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<C> Write for EventedDescriptor<C>
where
    C: ::DescriptorCloser,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
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
    type Item = Bytes;
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
                    return Ok(Async::Ready(Some(packet.freeze())));
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
                        Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Failed to send whole datagram",
                        ))
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
    pub fn pop_split_channels(
        &mut self,
    ) -> Option<(
        impl Sink<SinkItem = Bytes, SinkError = io::Error>,
        impl Stream<Item = Bytes, Error = io::Error>,
    )> {
        self.queues.pop().map(|q| AsyncDescriptor::from(q).split())
    }
}
