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
    pub fn get_flags(&self) -> libc::c_short {
        unsafe { self.ifr_ifru.ifru_flags }
    }

    pub fn set_flags(&mut self, flags: libc::c_short) {
        self.ifr_ifru.ifru_flags = flags;
    }
}