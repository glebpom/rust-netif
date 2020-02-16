use cfg_if::cfg_if;
use libc;
use std::{io, mem};

#[allow(non_camel_case_types)]
pub type caddr_t = *mut libc::c_char;

#[derive(Copy, Clone)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct ifreq {
    pub ifr_name: crate::IfName,
    pub ifr_ifru: ifr_ifru,
}

impl ifreq {
    pub fn set_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifr_name, name)
    }

    pub fn get_name(&self) -> io::Result<String> {
        get_name!(self.ifr_name)
    }
}

impl ifaliasreq {
    pub fn set_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifra_name, name)
    }

    pub fn get_name(&self) -> io::Result<String> {
        get_name!(self.ifra_name)
    }

    pub fn from_name(name: &str) -> io::Result<ifaliasreq> {
        let mut req: ifaliasreq = unsafe { mem::zeroed() };
        req.set_name(name)?;
        Ok(req)
    }
}

cfg_if! {
    if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        mod apple;
        pub use self::apple::*;
    } else if #[cfg(any(target_os = "openbsd", target_os = "netbsd",
                        target_os = "bitrig"))] {
        mod netbsdlike;
        pub use self::netbsdlike::*;
    } else if #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))] {
        mod freebsdlike;
        pub use self::freebsdlike::*;
    } else {
        // Unknown target_os
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ifaliasreq() {
        let _req = ifaliasreq::from_name("en0").unwrap();
    }
}
