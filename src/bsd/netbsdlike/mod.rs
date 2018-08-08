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
    pub fn get_flags(&self) -> libc::c_short {
        unsafe { self.ifr_ifru.ifru_flags }
    }

    pub fn set_flags(&mut self, flags: libc::c_short) {
        self.ifr_ifru.ifru_flags = flags;
    }
}