use parking_lot::Mutex;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::mem;
use std::os::windows::io::*;
use std::slice;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

use mio::windows;
use mio::{Evented, Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use miow::iocp::CompletionStatus;
use winapi::shared::winerror::*;
use winapi::um::fileapi::*;
use winapi::um::handleapi::*;
use winapi::um::ioapiset::*;
use winapi::um::minwinbase::*;

mod from_raw_arc;
mod handle;

use self::from_raw_arc::FromRawArc;
use self::handle::Handle;

macro_rules! offset_of {
    ($t:ty, $($field:ident).+) => (
        &(*(0 as *const $t)).$($field).+ as *const _ as usize
    )
}

macro_rules! overlapped2arc {
    ($e:expr, $t:ty, $($field:ident).+) => ({
        let offset = offset_of!($t, $($field).+);
        debug_assert!(offset < mem::size_of::<$t>());
        FromRawArc::from_raw(($e as usize - offset) as *mut $t)
    })
}

fn would_block() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "would block")
}

/// Representation of a named pipe on Windows.
///
/// This structure internally contains a `HANDLE` which represents the named
/// pipe, and also maintains state associated with the mio event loop and active
/// I/O operations that have been scheduled to translate IOCP to a readiness
/// model.
pub struct AsyncFile {
    registered: AtomicBool,
    ready_registration: Registration,
    poll_registration: windows::Binding,
    inner: FromRawArc<Inner>,
}

struct Inner {
    handle: Handle,
    readiness: SetReadiness,

    read: windows::Overlapped,
    write: windows::Overlapped,

    io: Mutex<Io>,
}

struct Io {
    read: State,
    write: State,
}

enum State {
    None,
    Pending(Vec<u8>, usize),
    Ok(Vec<u8>, usize),
    Err(io::Error),
}

fn _assert_kinds() {
    fn _assert_send<T: Send>() {}
    fn _assert_sync<T: Sync>() {}
    _assert_send::<AsyncFile>();
    _assert_sync::<AsyncFile>();
}

impl AsyncFile {
    fn registered(&self) -> bool {
        self.registered.load(SeqCst)
    }
}

impl Read for AsyncFile {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        <&AsyncFile as Read>::read(&mut &*self, buf)
    }
}

impl Write for AsyncFile {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        <&AsyncFile as Write>::write(&mut &*self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        <&AsyncFile as Write>::flush(&mut &*self)
    }
}

impl<'a> Read for &'a AsyncFile {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Make sure we're registered
        if !self.registered() {
            return Err(would_block());
        };

        let mut state = self.inner.io.lock();
        match mem::replace(&mut state.read, State::None) {
            // In theory not possible with `ready_registration` checked above,
            // but return would block for now.
            State::None => Err(would_block()),

            // A read is in flight, still waiting for it to finish
            State::Pending(buf, amt) => {
                state.read = State::Pending(buf, amt);
                Err(would_block())
            }

            // We previously read something into `data`, try to copy out some
            // data. If we copy out all the data schedule a new read and
            // otherwise store the buffer to get read later.
            State::Ok(data, cur) => {
                let n = {
                    let mut remaining = &data[cur..];
                    try!(remaining.read(buf))
                };
                let next = cur + n;
                if next != data.len() {
                    state.read = State::Ok(data, next);
                } else {
                    Inner::schedule_read(&self.inner, &mut state);
                }
                Ok(n)
            }

            // Looks like an in-flight read hit an error, return that here while
            // we schedule a new one.
            State::Err(e) => {
                Inner::schedule_read(&self.inner, &mut state);
                if e.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                    Ok(0)
                } else {
                    Err(e)
                }
            }
        }
    }
}

impl<'a> Write for &'a AsyncFile {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Make sure we're registered
        if !self.registered() {
            return Err(would_block());
        }

        // Make sure there's no writes pending
        let mut io = self.inner.io.lock();
        match io.write {
            State::None => {}
            _ => return Err(would_block()),
        }

        // Move `buf` onto the heap and fire off the write
        //
        // TODO: need to be smarter about buffer management here
        Inner::schedule_write(&self.inner, buf.to_vec(), 0, &mut io);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // TODO: `FlushFileBuffers` somehow?
        Ok(())
    }
}

impl Evented for AsyncFile {
    #[inline]
    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        // First, register the handle with the event loop
        unsafe {
            let mut info: BY_HANDLE_FILE_INFORMATION = mem::zeroed();

            try!(self.poll_registration.register_handle(&self.inner.handle, token, poll));
        }
        try!(poll.register(&self.ready_registration, token, interest, opts));
        self.registered.store(true, SeqCst);
        Inner::post_register(&self.inner);
        Ok(())
    }

    #[inline]
    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        // Validate `Poll` and that we were previously registered
        unsafe {
            try!(self.poll_registration.reregister_handle(&self.inner.handle, token, poll));
        }

        // At this point we should for sure have `ready_registration` unless
        // we're racing with `register` above, so just return a bland error if
        // the borrow fails.
        try!(poll.reregister(&self.ready_registration, token, interest, opts));

        Inner::post_register(&self.inner);

        Ok(())
    }

    #[inline]
    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        // Validate `Poll` and deregister ourselves
        unsafe {
            try!(self.poll_registration.deregister_handle(&self.inner.handle, poll));
        }
        poll.deregister(&self.ready_registration)
    }
}

impl AsRawHandle for AsyncFile {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.inner.handle.as_raw_handle()
    }
}

// impl FromRawHandle for AsyncFile {
//     unsafe fn from_raw_handle(handle: RawHandle) -> AsyncFile {
//         let (r, s) = Registration::new2();
//         AsyncFile {
//             registered: AtomicBool::new(false),
//             ready_registration: r,
//             poll_registration: windows::Binding::new(),
//             inner: FromRawArc::new(Inner {
//                 handle: Handle::from_raw_handle(handle),
//                 readiness: s,
//                 read: windows::Overlapped::new(mem::transmute(read_done as fn(_))),
//                 write: windows::Overlapped::new(mem::transmute(write_done as fn(_))),
//                 io: Mutex::new(Io {
//                     read: State::None,
//                     write: State::None,
//                 }),
//             }),
//         }
//     }
// }

impl From<Handle> for AsyncFile {
    fn from(handle: Handle) -> AsyncFile {
        let (r, s) = Registration::new2();
        AsyncFile {
            registered: AtomicBool::new(false),
            ready_registration: r,
            poll_registration: windows::Binding::new(),
            inner: FromRawArc::new(Inner {
                handle,
                readiness: s,
                read: unsafe { windows::Overlapped::new(mem::transmute(read_done as fn(_))) },
                write: unsafe { windows::Overlapped::new(mem::transmute(write_done as fn(_))) },
                io: Mutex::new(Io {
                    read: State::None,
                    write: State::None,
                }),
            }),
        }
    }
}

impl From<File> for AsyncFile {
    fn from(file: File) -> AsyncFile {
        let handle = file.into_raw_handle();
        unsafe { Handle::from_raw_handle(handle) }.into()
    }
}

impl fmt::Debug for AsyncFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.handle.fmt(f)
    }
}

impl Drop for AsyncFile {
    fn drop(&mut self) {
        unsafe {
            self.inner.force_drop();
        }
    }
}

impl Inner {
    /// Schedules a read to happen in the background, executing an overlapped
    /// operation.
    ///
    /// This function returns `true` if a normal error happens or if the read
    /// is scheduled in the background.
    ///
    /// FIX:
    /// If the pipe is no longer connected
    /// (ERROR_PIPE_LISTENING) then `false` is returned and no read is
    /// scheduled.
    fn schedule_read(me: &FromRawArc<Inner>, io: &mut Io) -> bool {
        // Check to see if a read is already scheduled/completed
        match io.read {
            State::None => {}
            _ => return true,
        }

        // Turn off our read readiness
        let ready = me.readiness.readiness();
        me.readiness.set_readiness(ready & !Ready::readable()).expect("event loop seems gone");

        // Allocate a buffer and schedule the read.
        //
        // TODO: need to be smarter about buffer management here
        let mut buf = Vec::with_capacity(8 * 1024);
        let e = unsafe {
            let overlapped = me.read.as_mut_ptr() as *mut _;
            let slice = slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.capacity());
            me.handle.read_overlapped(slice, overlapped)
        };

        match e {
            // See `connect` above for the rationale behind `forget`
            Ok(e) => {
                io.read = State::Pending(buf, 0); // 0 is ignored on read side
                mem::forget(me.clone());
                true
            }

            // If some other error happened, though, we're now readable to give
            // out the error.
            Err(e) => {
                io.read = State::Err(e);
                me.readiness.set_readiness(ready | Ready::readable()).expect("event loop still seems gone");
                true
            }
        }
    }

    fn schedule_write(me: &FromRawArc<Inner>, buf: Vec<u8>, pos: usize, io: &mut Io) {
        // Very similar to `schedule_read` above, just done for the write half.
        let ready = me.readiness.readiness();
        me.readiness.set_readiness(ready & !Ready::writable()).expect("event loop seems gone");

        let e = unsafe {
            let overlapped = me.write.as_mut_ptr() as *mut _;
            me.handle.write_overlapped(&buf[pos..], overlapped)
        };

        match e {
            // See `connect` above for the rationale behind `forget`
            Ok(e) => {
                io.write = State::Pending(buf, pos);
                mem::forget(me.clone())
            }
            Err(e) => {
                io.write = State::Err(e);
                me.add_readiness(Ready::writable());
            }
        }
    }

    fn add_readiness(&self, ready: Ready) {
        self.readiness.set_readiness(ready | self.readiness.readiness()).expect("event loop still seems gone");
    }

    fn post_register(me: &FromRawArc<Inner>) {
        let mut io = me.io.lock();
        if Inner::schedule_read(&me, &mut io) {
            if let State::None = io.write {
                me.add_readiness(Ready::writable());
            }
        }
    }
}

fn read_done(status: &OVERLAPPED_ENTRY) {
    let status = CompletionStatus::from_entry(status);

    // Acquire the `FromRawArc<Inner>`. Note that we should be guaranteed that
    // the refcount is available to us due to the `mem::forget` in
    // `schedule_read` above.
    let me = unsafe { overlapped2arc!(status.overlapped(), Inner, read) };

    // Move from the `Pending` to `Ok` state.
    let mut io = me.io.lock();
    let mut buf = match mem::replace(&mut io.read, State::None) {
        State::Pending(buf, _) => buf,
        _ => unreachable!(),
    };
    unsafe {
        match me.handle.result(status.overlapped()) {
            Ok(n) => {
                debug_assert_eq!(status.bytes_transferred() as usize, n);
                buf.set_len(status.bytes_transferred() as usize);
                io.read = State::Ok(buf, 0);
            }

            Err(e) => {
                debug_assert_eq!(status.bytes_transferred(), 0);
                io.read = State::Err(e);
            }
        }
    }

    // Flag our readiness that we've got data.
    me.add_readiness(Ready::readable());
}

fn write_done(status: &OVERLAPPED_ENTRY) {
    let status = CompletionStatus::from_entry(status);
    // Acquire the `FromRawArc<Inner>`. Note that we should be guaranteed that
    // the refcount is available to us due to the `mem::forget` in
    // `schedule_write` above.
    let me = unsafe { overlapped2arc!(status.overlapped(), Inner, write) };

    // Make the state change out of `Pending`. If we wrote the entire buffer
    // then we're writable again and otherwise we schedule another write.
    let mut io = me.io.lock();
    let (buf, pos) = match mem::replace(&mut io.write, State::None) {
        State::Pending(buf, pos) => (buf, pos),
        _ => unreachable!(),
    };

    unsafe {
        match me.handle.result(status.overlapped()) {
            Ok(n) => {
                debug_assert_eq!(status.bytes_transferred() as usize, n);
                let new_pos = pos + (status.bytes_transferred() as usize);
                if new_pos == buf.len() {
                    me.add_readiness(Ready::writable());
                } else {
                    Inner::schedule_write(&me, buf, new_pos, &mut io);
                }
            }
            Err(e) => {
                debug_assert_eq!(status.bytes_transferred(), 0);
                io.write = State::Err(e);
                me.add_readiness(Ready::writable());
            }
        }
    }
}
