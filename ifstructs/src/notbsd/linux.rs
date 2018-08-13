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
    pub ifr_slave: ::IfName,
    pub ifr_newname: ::IfName,
    pub ifr_data: *mut libc::c_char,
}

#[repr(C)]
pub struct ifreq {
    pub ifr_name: ::IfName,
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
        ::IfFlags::from_bits_truncate(i32::from(self.ifr_ifru.ifr_flags))
    }

    /// Enable passed flags
    pub unsafe fn set_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifr_flags = flags.bits() as i16;
    }

    /// Enable passed flags
    pub unsafe fn set_raw_flags(&mut self, raw_flags: libc::c_short) {
        self.ifr_ifru.ifr_flags = raw_flags;
    }

    pub unsafe fn set_addr(&mut self, addr: libc::sockaddr) {
        self.ifr_ifru.ifr_addr = addr;
    }

    pub unsafe fn set_iface_index(&mut self, idx: libc::c_int) {
        self.ifr_ifru.ifr_ifindex = idx;
    }

    pub unsafe fn get_iface_index(&mut self) -> libc::c_int {
        self.ifr_ifru.ifr_ifindex
    }
}

#[repr(C)]
pub struct rtentry {
    rt_pad1: libc::c_ulong,
    rt_dst: libc::sockaddr,
    rt_gateway: libc::sockaddr,
    rt_genmask: libc::sockaddr,
    rt_flags: libc::c_ushort,
    rt_pad2: libc::c_short,
    rt_pad3: libc::c_ulong,
    rt_pad4: *const libc::c_void,
    rt_metric: libc::c_short,
    rt_dev: *mut libc::c_char,
    rt_mtu: libc::c_ulong,
    rt_window: libc::c_ulong,
    rt_irtt: libc::c_ushort,
}

bitflags! {
    pub struct RtFlags: libc::c_ushort {
        const RTF_UP        = 0x0001;
        const RTF_GATEWAY   = 0x0002;
        const RTF_HOST      = 0x0004;
        const RTF_REINSTATE = 0x0008;
        const RTF_DYNAMIC   = 0x0010;
        const RTF_MODIFIED  = 0x0020;
        const RTF_MTU       = 0x0040; //RTF_MTU alias
        const RTF_WINDOW    = 0x0080;
        const RTF_IRTT      = 0x0100;
        const RTF_REJECT    = 0x0200;
    }
}
