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
    /// Get flags
    pub unsafe fn get_flags(&self) -> ::IfFlags {
        ::IfFlags::from_bits_truncate(i32::from(self.ifr_ifru.ifru_flags))
    }

    /// Set flags
    pub unsafe fn set_flags(&mut self, flags: ::IfFlags) {
        self.ifr_ifru.ifru_flags = flags.bits() as i16;
    }

    /// Enable passed flags
    pub unsafe fn set_raw_flags(&mut self, raw_flags: libc::c_short) {
        self.ifr_ifru.ifru_flags = raw_flags;
    }


    pub unsafe fn set_addr(&mut self, addr: libc::sockaddr) {
        self.ifr_ifru.ifru_addr = addr;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifaliasreq {
    pub ifra_name: ::IfName,
    pub ifra_addr: libc::sockaddr,
    pub ifra_broadaddr: libc::sockaddr,
    pub ifra_mask: libc::sockaddr,
}

// bitflags! {
//     // https://github.com/apple/darwin-xnu/blob/master/bsd/net/route.h
//     pub struct Flags: i32 {
//         const RTF_UP = sys::RTF_UP;
//         const RTF_GATEWAY = sys::RTF_GATEWAY;
//         const RTF_HOST = sys::RTF_HOST;
//         const RTF_REJECT = sys::RTF_REJECT;
//         const RTF_DYNAMIC = sys::RTF_DYNAMIC;
//         const RTF_MODIFIED = sys::RTF_MODIFIED;
//         const RTF_DONE = sys::RTF_DONE;
//         const RTF_DELCLONE = sys::RTF_DELCLONE;
//         const RTF_CLONING = sys::RTF_CLONING;
//         const RTF_XRESOLVE = sys::RTF_XRESOLVE;
//         const RTF_LLINFO = sys::RTF_LLINFO;
//         const RTF_LLDATA = sys::RTF_LLDATA;
//         const RTF_STATIC = sys::RTF_STATIC;
//         const RTF_BLACKHOLE = sys::RTF_BLACKHOLE;
//         const RTF_NOIFREF = sys::RTF_NOIFREF;
//         const RTF_PROTO2 = sys::RTF_PROTO2;
//         const RTF_PROTO1 = sys::RTF_PROTO1;
//         const RTF_PRCLONING = sys::RTF_PRCLONING;
//         const RTF_WASCLONED = sys::RTF_WASCLONED;
//         const RTF_PROTO3 = sys::RTF_PROTO3;
//         const RTF_PINNED = sys::RTF_PINNED;
//         const RTF_LOCAL = sys::RTF_LOCAL;
//         const RTF_BROADCAST = sys::RTF_BROADCAST;
//         const RTF_MULTICAST = sys::RTF_MULTICAST;
//         const RTF_IFSCOPE = sys::RTF_IFSCOPE;
//         const RTF_CONDEMNED = sys::RTF_CONDEMNED;
//         const RTF_IFREF = sys::RTF_IFREF;
//         const RTF_PROXY = sys::RTF_PROXY;
//         const RTF_ROUTER = sys::RTF_ROUTER;
//         const RTF_DEAD = sys::RTF_DEAD;
//         const RTPRF_OURS = sys::RTPRF_OURS;
//     }
// }
