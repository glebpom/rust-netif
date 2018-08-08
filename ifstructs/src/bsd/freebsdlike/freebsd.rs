use libc;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifmap {
    pub mem_start: libc::c_ulong,
    pub mem_end: libc::c_ulong,
    pub base_addr: libc::c_ushort,
    pub irq: libc::c_uchar,
    pub dma: libc::c_uchar,
    pub port: libc::c_uchar,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifreq_buffer {
    pub length: libc::size_t,
    pub buffer: *mut libc::c_void,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifr_ifru {
    pub ifru_addr: libc::sockaddr,
    pub ifru_dstaddr: libc::sockaddr,
    pub ifru_broadaddr: libc::sockaddr,
    pub ifru_buffer: ifreq_buffer,
    pub ifru_flags: [libc::c_short; 2],
    pub ifru_index: libc::c_short,
    pub ifru_jid: libc::c_int,
    pub ifru_metric: libc::c_int,
    pub ifru_mtu: libc::c_int,
    pub ifru_phys: libc::c_int,
    pub ifru_media: libc::c_int,
    pub ifru_data: ::caddr_t,
    pub ifru_cap: [libc::c_int; 2],
    pub ifru_fib: libc::c_uint,
    pub ifru_vlan_pcp: libc::c_uchar,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifaliasreq {
    pub ifra_name: [u8; libc::IFNAMSIZ],
    pub ifra_addr: libc::sockaddr,
    pub ifra_broadaddr: libc::sockaddr,
    pub ifra_mask: libc::sockaddr,
    pub ifra_vhid: libc::c_int,
}

