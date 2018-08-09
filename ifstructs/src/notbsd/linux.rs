use libc;
use std::io;

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

impl ::ifreq {
    pub fn set_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifr_name, name)
    }

    pub fn get_name(&self) -> io::Result<String> {
        get_name!(self.ifr_name)
    }

    /// Get flags
    pub unsafe fn get_flags(&self) -> ::IfFlags {
        ::IfFlags::from_bits_truncate(self.ifr_ifru.ifr_flags)
    }

    /// Enable passed flags
    pub unsafe fn insert_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifr_flags |= flags.bits();
    }

    /// Enable passed flags
    pub unsafe fn remove_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifr_flags &= !flags.bits();
    }
}

bitflags! {
    pub struct IfFlags: libc::c_short {
        const IFF_RUNNING = libc::IFF_RUNNING as libc::c_short;
        const IFF_UP = libc::IFF_UP as libc::c_short;
        const IFF_NO_PI = libc::IFF_NO_PI as libc::c_short;
        const IFF_MULTI_QUEUE = libc::IFF_MULTI_QUEUE as libc::c_short;
        const IFF_TUN = libc::IFF_TUN as libc::c_short;
        const IFF_TAP = libc::IFF_TAP as libc::c_short;
    }
}

