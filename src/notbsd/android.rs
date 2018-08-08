use libc;

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifs_ifsu {
    //For now it's all pointers to void
    raw_hdlc: *mut libc::c_void,
    cisco: *mut libc::c_void,
    fr: *mut libc::c_void,
    fr_pvc: *mut libc::c_void,
    fr_pvc_info: *mut libc::c_void,
    sync: *mut libc::c_void,
    te1: *mut libc::c_void,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct if_settings {
    type_: libc::c_uint,
    size: libc::c_uint,
    ifs_ifsu: ifs_ifsu,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifrn_ifru {
    pub ifru_addr: libc::sockaddr,
    pub ifru_dstaddr: libc::sockaddr,
    pub ifru_broadaddr: libc::sockaddr,
    pub ifru_netmask: libc::sockaddr,
    pub ifru_hwaddr: libc::sockaddr,
    pub ifru_flags: libc::c_short,
    pub ifru_ivalue: libc::c_int,
    pub ifru_mtu: libc::c_int,
    pub ifru_map: ::ifmap,
    pub ifrn_slave: [u8; libc::IFNAMSIZ],
    pub ifrn_newname: [u8; libc::IFNAMSIZ],
    pub ifru_data: *mut libc::c_void,
    pub ifru_settings: if_settings,
}

#[repr(C)]
pub union ifr_ifrn {
    pub ifrn_name: [u8; libc::IFNAMSIZ],
}

#[repr(C)]
pub struct ifreq {
    pub ifrn_ifrn: ifr_ifrn,
    pub ifrn_ifru: ifrn_ifru,
}
