cfg_if! {
    if #[cfg(target_os = "openbsd")] {
        mod openbsd;
        pub use self::openbsd::*;
    } else if #[cfg(target_os = "bitrig")] {
        mod bitrig;
        pub use self::bitrig::*;
    } else {
        // Unknown target_os
    }
}

pub union ifra_ifrau {
    pub ifrau_addr: libc::sockaddr,
    pub ifrau_align: libc::int,
}

struct ifaliasreq {
    pub ifra_name: [u8; libc::IFNAMSIZ],
    pub ifra_ifrau: ifra_ifrau,
    pub ifra_dstaddr: libc::sockaddr,
    pub ifra_mask: libc::sockaddr,
}

