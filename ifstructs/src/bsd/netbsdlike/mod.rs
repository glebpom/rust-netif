use libc;

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

impl ::ifreq {
   /// Get flags
    pub unsafe fn get_flags(&self) -> ::IfFlags {
        ::IfFlags::from_bits_truncate(self.ifr_ifru.ifru_flags )
    }

    /// Enable passed flags
    pub unsafe fn insert_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags  |= flags.bits();
    }

    /// Enable passed flags
    pub unsafe fn remove_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags  &= !flags.bits();
    }
}