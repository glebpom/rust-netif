use libc;

//FIXME: consider 32 bits
impl ::ifreq {
    /// Get flags
    pub unsafe fn get_flags(&self) -> ::IfFlags {
        ::IfFlags::from_bits_truncate(i32::from(self.ifr_ifru.ifru_flags[0]))
    }


    /// Enable passed flags
    pub unsafe fn set_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags[0] = flags.bits() as i16;
    }

    /// Enable passed flags
    pub unsafe fn set_raw_flags(&mut self, raw_flags: libc::c_short) {
        self.ifr_ifru.ifru_flags[0] = raw_flags;
    }

    pub unsafe fn set_addr(&mut self, addr: libc::sockaddr) {
        self.ifr_ifru.ifru_addr = addr;
    }
}

cfg_if! {
    if #[cfg(target_os = "freebsd")] {
        mod freebsd;
        pub use self::freebsd::*;
    } else {
        // ...
    }
}