cfg_if! {
    if #[cfg(target_os = "netbsd")] {
        mod netbsd;
        pub use self::netbsd::*;
    } else if #[cfg(any(target_os = "openbsd", target_os = "bitrig"))] {
        mod openbsdlike;
        pub use self::openbsdlike::*;
    } else {
        // Unknown target_os
    }
}

use libc;

impl ::ifreq {
    /// Get flags
    pub unsafe fn get_flags(&self) -> ::IfFlags {
        ::IfFlags::from_bits_truncate(i32::from(self.ifr_ifru.ifru_flags))
    }

    /// Enable passed flags
    pub unsafe fn set_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags = flags.bits() as i16;
    }

    /// Enable passed flags
    pub unsafe fn set_raw_flags(&mut self, raw_flags: libc::c_short) {
        self.ifr_ifru.ifru_flags = raw_flags;
    }


    pub unsafe fn set_addr(&mut self, addr: libc::sockaddr) {
        self.ifr_ifru.ifru_addr = addr;
    }
}
