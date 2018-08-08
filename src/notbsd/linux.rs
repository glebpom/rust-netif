use libc;

#[repr(C)]
pub union ifr_ifru {
    pub ifr_addr: libc::sockaddr,
    pub ifr_dstaddr: libc::sockaddr,
    pub ifr_broadaddr: libc::sockaddr,
    pub ifr_netmask: libc::sockaddr,
    pub ifr_hwaddr: libc::sockaddr,
    pub ifr_flags: libc::c_short,
    pub ifr_ifindex: libc::c_int,
    pub ifr_metric: libc::c_int,
    pub ifr_mtu: libc::c_int,
    pub ifr_map: ::ifmap,
    pub ifr_slave: [libc::c_char; libc::IFNAMSIZ],
    pub ifr_newname: [libc::c_char; libc::IFNAMSIZ],
    pub ifr_data: *mut libc::c_char,
}

#[repr(C)]
pub struct ifreq {
    pub ifr_name: [u8; libc::IFNAMSIZ],
    pub ifr_ifru: ifr_ifru,
}
