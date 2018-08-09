use libc;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifru_b {
    pub b_buflen: libc::uint32_t,
    pub b_buf: *mut libc::c_void,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifr_ifru {
    pub ifru_addr: libc::sockaddr,
    pub ifru_dstaddr: libc::sockaddr,
    pub ifru_broadaddr: libc::sockaddr, 
    pub ifru_space: libc::sockaddr_storage,
    pub ifru_flags: libc::c_short,
    pub ifru_addrflags: libc::c_int,
    pub ifru_metric: libc::c_int,
    pub ifru_mtu: libc::c_int,
    pub ifru_dlt: libc::c_int,
    pub ifru_value: libc::c_uint,
    pub ifru_data: *mut libc::c_void,
    pub ifru_b: ifru_b,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifaliasreq {
    pub ifra_name: [u8; libc::IFNAMSIZ],
    pub ifra_addr: libc::sockaddr,
    pub ifra_broadaddr: libc::sockaddr,
    pub ifra_mask: libc::sockaddr,
}

bitflags! {
    pub struct IfFlags: libc::c_short {
        const IFF_RUNNING = libc::IFF_RUNNING as libc::c_short;
        const IFF_UP = libc::IFF_UP as libc::c_short;
    }
}
