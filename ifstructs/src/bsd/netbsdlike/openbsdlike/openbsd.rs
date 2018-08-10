use libc;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub union ifr_ifru {
    pub ifru_addr: libc::sockaddr,
    pub ifru_dstaddr: libc::sockaddr,
    pub ifru_broadaddr: libc::sockaddr,
    pub ifru_flags: libc::c_short,
    pub ifru_metric: libc::c_int,
    pub ifru_vnetid: libc::int64_t,
    pub ifru_media: libc::uint64_t,
    pub ifru_data: ::caddr_t,
    pub ifru_index: libc::c_uint,
}
