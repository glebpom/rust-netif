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

use bytes::{BufMut, Bytes, BytesMut};
use futures::{ready, Sink, Stream, StreamExt};
use futures::task::{Context, Poll};
use mio;
use mio::event::Evented;
#[cfg(unix)]
use mio::unix::EventedFd;
#[cfg(windows)]
use parking_lot::Mutex;
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::PollEvented;
use tokio::macros::support::Pin;

#[cfg(windows)]
use impls::r#async::AsyncFile;
#[cfg(windows)]
use impls::set_iface_status;

use crate::{MTU, RESERVE_AT_ONCE};

#[cfg(unix)]
impl<C> Evented for EventedDescriptor<C>
    where
        C: crate::DescriptorCloser,
{
    fn register(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.io().as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
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
    fn register(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        self.inner.register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        self.inner.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        self.inner.deregister(poll)
    }
}

#[cfg(unix)]
pub struct EventedDescriptor<C: crate::DescriptorCloser>(crate::Descriptor<C>);

#[cfg(windows)]
pub struct EventedDescriptor<C: ::DescriptorCloser> {
    inner: AsyncFile,
    _closer: PhantomData<C>,
    info: Arc<Mutex<VirtualInterfaceInfo>>,
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
        disable
            .arg("interface")
            .arg("set")
            .arg("interface")
            .arg(&iface_name)
            .arg("admin=disable");
        eprintln!("Executing command {:?}", disable);
        eprintln!("Result: {:?}", disable.output());
        disable
            .output()
            .expect("Could not re-enable TAP iface: disable failed");
        let mut enable = Command::new("netsh");
        enable
            .arg("interface")
            .arg("set")
            .arg("interface")
            .arg(&iface_name)
            .arg("admin=enable");
        eprintln!("Executing command {:?}", enable);
        eprintln!("Result: {:?}", enable.output());
    }
}

impl<C> EventedDescriptor<C>
    where
        C: crate::DescriptorCloser,
{
    #[cfg(unix)]
    fn io_mut(&mut self) -> &mut crate::Descriptor<C> {
        &mut self.0
    }

    #[cfg(unix)]
    fn io(&self) -> &crate::Descriptor<C> {
        &self.0
    }

    #[cfg(windows)]
    fn io_mut(&mut self) -> &mut AsyncFile {
        &mut self.inner
    }
}

#[cfg(unix)]
impl<C> From<crate::Descriptor<C>> for EventedDescriptor<C>
    where
        C: crate::DescriptorCloser,
{
    fn from(d: crate::Descriptor<C>) -> EventedDescriptor<C> {
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
        C: crate::DescriptorCloser,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.io_mut().read(buf)
    }
}

impl<C> Write for EventedDescriptor<C>
    where
        C: crate::DescriptorCloser,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.io_mut().flush()
    }
}

pin_project! {
    pub struct AsyncDescriptor<C>
        where
            C: crate::DescriptorCloser,
    {
        #[pin]
        inner: PollEvented<EventedDescriptor<C>>,
        incoming: Option<Bytes>,
        outgoing: BytesMut,
    }
}
impl<C> From<PollEvented<EventedDescriptor<C>>> for AsyncDescriptor<C>
    where
        C: crate::DescriptorCloser,
{
    fn from(f: PollEvented<EventedDescriptor<C>>) -> AsyncDescriptor<C> {
        AsyncDescriptor {
            inner: f,
            incoming: None,
            outgoing: BytesMut::with_capacity(RESERVE_AT_ONCE),
        }
    }
}

impl<C> Stream for AsyncDescriptor<C>
    where
        C: crate::DescriptorCloser,
{
    type Item = Result<BytesMut, io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let pin = self.get_mut();

        let n = unsafe {
            let n = {
                let b = pin.outgoing.bytes_mut();

                // Convert to `&mut [u8]`
                let b = &mut *(b as *mut _ as *mut [u8]);

                let n = ready!(Pin::new(&mut pin.inner).poll_read(cx, b))?;
                assert!(n <= b.len(), "Bad AsyncRead implementation, more bytes were reported as read than the buffer can hold");
                n
            };

            pin.outgoing.advance_mut(n);

            n
        };

        let frame = pin.outgoing.split_to(n);
        let cur_capacity = pin.outgoing.capacity();
        if cur_capacity < MTU {
            pin.outgoing.reserve(RESERVE_AT_ONCE);
        }
        Poll::Ready(Some(Ok(frame)))
    }
}

#[must_use = "sinks do nothing unless polled"]
impl<C> Sink<Bytes> for AsyncDescriptor<C>
    where
        C: crate::DescriptorCloser,
{
    type Error = io::Error;


    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.incoming.is_some() {
            match self.poll_flush(cx)? {
                Poll::Ready(()) => {}
                Poll::Pending => return Poll::Pending,
            }
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, frame: Bytes) -> Result<(), Self::Error> {
        let pin = self.get_mut();

        pin.incoming = Some(frame);

        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.incoming.is_none() {
            return Poll::Ready(Ok(()));
        }

        let Self {
            ref mut inner,
            ref mut incoming,
            ..
        } = *self;

        let incoming = incoming.as_mut().unwrap();

        let n = ready!(Pin::new(inner).poll_write(cx, incoming))?;

        let wrote_all = n == incoming.len();
        self.incoming = None;

        let res = if wrote_all {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to write entire datagram to socket",
            )
                .into())
        };

        Poll::Ready(res)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }
}

impl<C> crate::Virtualnterface<PollEvented<EventedDescriptor<C>>>
    where
        C: crate::DescriptorCloser,
{
    pub fn pop_split_channels(
        &mut self,
    ) -> Option<(
        impl Sink<Bytes, Error=io::Error>,
        impl Stream<Item=Result<BytesMut, io::Error>>,
    )> {
        if let Some(q) = self.queues.pop() {
            Some(AsyncDescriptor::from(q).split())
        } else {
            None
        }
    }
}
