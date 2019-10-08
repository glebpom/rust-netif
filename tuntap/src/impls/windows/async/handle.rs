use std::cmp;
use std::io;
use std::ptr;

use winapi::shared::minwindef::*;
use winapi::shared::winerror::*;
use winapi::um::fileapi::*;
use winapi::um::handleapi::*;
use winapi::um::ioapiset::*;
use winapi::um::minwinbase::*;
use winapi::um::processthreadsapi::*;
use winapi::um::winnt::*;

use std::os::windows::io::{AsRawHandle, FromRawHandle};

pub(crate) fn cvt(i: BOOL) -> io::Result<BOOL> {
    if i == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(i)
    }
}

#[derive(Debug)]
pub struct Handle(HANDLE);

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Handle {
    pub fn into_raw(self) -> HANDLE {
        use std::mem;

        let ret = self.0;
        mem::forget(self);
        ret
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes = 0;
        let len = cmp::min(buf.len(), <DWORD>::max_value() as usize) as DWORD;
        try!(cvt(unsafe {
            WriteFile(
                self.0,
                buf.as_ptr() as *const _,
                len,
                &mut bytes,
                0 as *mut _,
            )
        }));
        Ok(bytes as usize)
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut bytes = 0;
        let len = cmp::min(buf.len(), <DWORD>::max_value() as usize) as DWORD;
        try!(cvt(unsafe {
            ReadFile(
                self.0,
                buf.as_mut_ptr() as *mut _,
                len,
                &mut bytes,
                0 as *mut _,
            )
        }));
        Ok(bytes as usize)
    }

    pub unsafe fn read_overlapped(
        &self,
        buf: &mut [u8],
        overlapped: *mut OVERLAPPED,
    ) -> io::Result<Option<usize>> {
        let len = cmp::min(buf.len(), <DWORD>::max_value() as usize) as DWORD;
        let mut bytes = 0;
        let res = cvt({
            ReadFile(
                self.0,
                buf.as_mut_ptr() as *mut _,
                len,
                &mut bytes,
                overlapped,
            )
        });
        match res {
            Ok(_) => Ok(Some(bytes as usize)),
            Err(ref e) if e.raw_os_error() == Some(ERROR_IO_PENDING as i32) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub unsafe fn write_overlapped(
        &self,
        buf: &[u8],
        overlapped: *mut OVERLAPPED,
    ) -> io::Result<Option<usize>> {
        let len = cmp::min(buf.len(), <DWORD>::max_value() as usize) as DWORD;
        let mut bytes = 0;
        let res = cvt({
            WriteFile(
                self.0,
                buf.as_ptr() as *const _,
                len,
                &mut bytes,
                overlapped,
            )
        });
        match res {
            Ok(_) => Ok(Some(bytes as usize)),
            Err(ref e) if e.raw_os_error() == Some(ERROR_IO_PENDING as i32) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // pub fn try_clone(&self) -> io::Result<Handle> {
    //     let access = 0;
    //     let inherit = true;
    //     let options = DUPLICATE_SAME_ACCESS;

    //     let mut ret = 0 as HANDLE;
    //     cvt(unsafe {
    //         let cur_proc = GetCurrentProcess();
    //         DuplicateHandle(cur_proc, self.0, cur_proc, &mut ret, access, inherit as BOOL, options)
    //     })?;
    //     Ok(Handle::from_raw_handle(ret))
    // }

    pub unsafe fn result(&self, overlapped: *mut OVERLAPPED) -> io::Result<usize> {
        let mut transferred = 0;
        let r = GetOverlappedResult(self.as_raw_handle(), overlapped, &mut transferred, FALSE);
        if r == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(transferred as usize)
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

impl AsRawHandle for Handle {
    fn as_raw_handle(&self) -> HANDLE {
        self.0
    }
}

impl FromRawHandle for Handle {
    unsafe fn from_raw_handle(handle: HANDLE) -> Handle {
        Handle(handle)
    }
}
