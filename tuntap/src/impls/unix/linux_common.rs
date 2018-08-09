use ifstructs::ifreq;
use impls::unix::*;
use libc;

bitflags! {
    pub struct TunTapFlags: libc::c_short {
        const IFF_TUN = 0x0001;
        const IFF_TAP = 0x0002;
        const IFF_NAPI = 0x0010;
        const IFF_NAPI_FRAGS = 0x0020;
        const IFF_NO_PI = 0x1000;
        /// this flag has no real effect
        const IFF_ONE_QUEUE = 0x2000;
        const IFF_VNET_HDR = 0x4000;
        const IFF_TUN_EXCL = 0x8000;
        const IFF_MULTI_QUEUE = 0x0100;
        const IFF_ATTACH_QUEUE = 0x0200;
        const IFF_DETACH_QUEUE = 0x0400;
        /// read-only flag
        const IFF_PERSIST = 0x0800;
        const IFF_NOFILTER = 0x1000;
    }
}

ioctl_write_ptr!(tun_set_iff, b'T', 202, libc::c_int);
