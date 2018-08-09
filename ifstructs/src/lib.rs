extern crate libc;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate bitflags;

#[cfg(test)]
#[macro_use]
extern crate nix;

#[macro_use]
mod macros;

use std::{io, mem};

cfg_if! {
    if #[cfg(any(target_os = "linux",
                 target_os = "android",
                 target_os = "fuchsia"))] {
        mod notbsd;
        pub use self::notbsd::*;
    } else if #[cfg(any(target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "dragonfly",
                        target_os = "openbsd",
                        target_os = "netbsd",
                        target_os = "bitrig"))] {
        mod bsd;
        pub use self::bsd::*;
    } else {
        // Unknown target_os
    }
}

impl ifreq {
    pub fn from_name(name: &str) -> io::Result<ifreq> {
        let mut req: ifreq = unsafe { mem::zeroed() };
        req.set_name(name)?;
        Ok(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io;

    #[test]
    fn test_get_iface_flags() {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        let request_code = 0x8913; // #define SIOCGIFFLAGS	0x8913		/* get flags			*/
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        let request_code = request_code_readwrite!(b'i', 17, mem::size_of::<ifreq>());

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let default_iface = "lo0";
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let default_iface = "lo";

        let iface_name = env::var("TEST_IFACE").unwrap_or(default_iface.to_owned());

        let mut req = ifreq::from_name(&iface_name).unwrap();

        let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
        if sock < 0 {
            panic!("Socket error {:?}", io::Error::last_os_error());
        }

        let res = unsafe { libc::ioctl(sock, request_code, &mut req) };
        if res < 0 {
            panic!(
                "SIOCGIFFLAGS failed with error on device '{}': {:?}",
                iface_name,
                io::Error::last_os_error()
            );
        }

        let flags = unsafe { req.get_flags() };

        assert!(flags.contains(IfFlags::IFF_UP));
    }
}
