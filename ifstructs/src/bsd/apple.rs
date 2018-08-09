use libc;

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifr_ifru {
    pub ifru_addr: libc::sockaddr,
    pub ifru_dstaddr: libc::sockaddr,
    pub ifru_broadaddr: libc::sockaddr,
    pub ifru_flags: libc::c_short,
    pub ifru_metric: libc::c_int,
    pub ifru_mtu: libc::c_int,
    pub ifru_phys: libc::c_int,
    pub ifru_media: libc::c_int,
    pub ifru_data: ::caddr_t,
}

impl ::ifreq {
    /// Get flags
    pub unsafe fn get_flags(&self) -> ::IfFlags {
        ::IfFlags::from_bits_truncate(i32::from(self.ifr_ifru.ifru_flags))
    }

    /// Set flags
    pub unsafe fn set_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags = flags.bits() as i16;
    }

    /// Enable passed flags
    pub unsafe fn set_raw_flags(&mut self, raw_flags: libc::c_short) {
        self.ifr_ifru.ifru_flags = raw_flags;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifaliasreq {
    pub ifra_name: [u8; libc::IFNAMSIZ],
    pub ifra_addr: libc::sockaddr,
    pub ifra_broadaddr: libc::sockaddr,
    pub ifra_mask: libc::sockaddr,
}
