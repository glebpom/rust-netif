extern crate libc;
#[macro_use]
extern crate cfg_if;

#[cfg(test)]
#[macro_use]
extern crate nix;

#[macro_use]
mod macros;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::mem;


    #[test]
    fn test_get_iface_flags() {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        let request_code = libc::SIOCGIFFLAGS;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        let request_code = request_code_readwrite!(b'i', 17, mem::size_of::<ifreq>());

        let mut req: ifreq = unsafe { mem::zeroed() };
        req.set_name("lo0").unwrap();

        let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
        if sock < 0 {
            panic!("Socket error {:?}", io::Error::last_os_error());
        }

        let res = unsafe { libc::ioctl(sock, request_code, &mut req) };
        if res < 0 {
            panic!(
                "SIOCGIFFLAGS failed with error {:?}",
                io::Error::last_os_error()
            );
        }

        let flags = req.get_flags();

        assert_ne!(i64::from(flags) & i64::from(libc::IFF_UP), 0);
    }
}
