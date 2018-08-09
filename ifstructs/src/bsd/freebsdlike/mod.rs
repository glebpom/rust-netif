use libc;

cfg_if! {
    if #[cfg(target_os = "freebsd")] {
        mod freebsd;
        pub use self::freebsd::*;
    } else if #[cfg(target_os = "dragonfly")] {
        mod dragonfly;
        pub use self::dragonfly::*;
    } else {
        // ...
    }
}

impl ::ifreq {
    /// Get flags
    pub unsafe fn get_flags(&self) -> ::IfFlags {
        ::IfFlags::from_bits_truncate(self.ifr_ifru.ifru_flags[0])
    }

    /// Enable passed flags
    pub unsafe fn insert_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags[0] |= flags.bits();
    }

    /// Enable passed flags
    pub unsafe fn remove_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags[0] &= !flags.bits();
    }
}

bitflags! {
    pub struct IfFlags: libc::c_short {
        const IFF_RUNNING = libc::IFF_RUNNING as libc::c_short;
        const IFF_UP = libc::IFF_UP as libc::c_short;
    }
}
