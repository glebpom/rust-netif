pub union ifra_ifrau {
    pub ifrau_addr: libc::sockaddr,
    pub ifrau_align: libc::int,
}

pub struct ifaliasreq {
    pub ifra_name: [u8; libc::IFNAMSIZ],
    pub ifra_ifrau: ifra_ifrau,
    pub ifra_broadaddr: libc::sockaddr, //originally ifra_dstaddr
    pub ifra_mask: libc::sockaddr,
}

cfg_if! {
    if #[cfg(target_os = "openbsd")] {
        mod openbsd;
        pub use self::openbsd::*;
    } else {
        // Unknown target_os
    }
}