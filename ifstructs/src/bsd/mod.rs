use libc;
use std::io;

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

#[allow(non_camel_case_types)]
pub type caddr_t = *mut libc::c_char;

#[repr(C)]
pub struct ifreq {
    pub ifr_name: [u8; libc::IFNAMSIZ],
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;
    
    #[test]
    fn test_ifaliasreq() {
        let _req: ifaliasreq = unsafe { mem::zeroed() };
    }
}
