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
    pub fn get_flags(&self) -> libc::c_short {
        unsafe { self.ifr_ifru.ifru_flags[0] }
    }

    pub fn set_flags(&mut self, flags: libc::c_short) {
        unsafe { self.ifr_ifru.ifru_flags[0] = flags };
    }
}