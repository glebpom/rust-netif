use libc;

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifr_ifru {
    pub ifru_addr: libc::sockaddr,
    pub ifru_dstaddr: libc::sockaddr,
    pub ifru_broadaddr: libc::sockaddr,
    pub ifru_flags: [libc::c_short; 2],
    pub ifru_metric: libc::c_int,
    pub ifru_mtu: libc::c_int,
    pub ifru_phys: libc::c_int,
    pub ifru_media: libc::c_int,
    pub ifru_data: *mut libc::c_void,
    pub ifru_cap: [libc::c_int; 2],
    pub ifru_pollcpu: libc::c_int,
    pub ifru_tsolen: libc::c_int,
}
