use libc;
use std::io;
use std::mem;

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
    pub ifra_name: ::IfName,
    pub ifra_addr: libc::sockaddr,
    pub ifra_broadaddr: libc::sockaddr,
    pub ifra_mask: libc::sockaddr,
    pub ifra_vhid: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifdrv {
    pub ifd_name: ::IfName,
    pub ifd_cmd: libc::c_ulong,
    pub ifd_len: libc::size_t,
    pub ifd_data: *mut libc::c_void,
}

impl ifdrv {
    pub fn set_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifd_name, name)
    }

    pub fn get_name(&self) -> io::Result<String> {
        get_name!(self.ifd_name)
    }

    pub fn from_name(name: &str) -> io::Result<ifdrv> {
        let mut req: ifdrv = unsafe { mem::zeroed() };
        req.set_name(name)?;
        Ok(req)
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifbreq {
    pub ifbr_ifsname: ::IfName,
    pub ifbr_ifsflags: libc::uint32_t,
    pub ifbr_stpflags: libc::uint32_t,
    pub ifbr_path_cost: libc::uint32_t,
    pub ifbr_portno: libc::uint8_t,
    pub ifbr_priority: libc::uint8_t,
    pub ifbr_proto: libc::uint8_t,
    pub ifbr_role: libc::uint8_t,
    pub ifbr_state: libc::uint8_t,
    pub ifbr_addrcnt: libc::uint32_t,
    pub ifbr_addrmax: libc::uint32_t,
    pub ifbr_addrexceeded: libc::uint32_t,
    pub pad: [libc::uint8_t; 32],
}

impl ifbreq {
    pub fn set_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifbr_ifsname, name)
    }

    pub fn get_name(&self) -> io::Result<String> {
        get_name!(self.ifbr_ifsname)
    }

    pub fn from_name(name: &str) -> io::Result<ifbreq> {
        let mut req: ifbreq = unsafe { mem::zeroed() };
        req.set_name(name)?;
        Ok(req)
    }
}

pub mod brcmd {
    use libc;

    pub const BRDGADD: libc::c_ulong = 0;
    pub const BRDGDEL: libc::c_ulong = 1;
    pub const BRDGGIFFLGS: libc::c_ulong = 2;
    pub const BRDGSIFFLGS: libc::c_ulong = 3;
    pub const BRDGSCACHE: libc::c_ulong = 4;
    pub const BRDGGCACHE: libc::c_ulong = 5;
    pub const BRDGGIFS: libc::c_ulong = 6;
    pub const BRDGRTS: libc::c_ulong = 7;
    pub const BRDGSADDR: libc::c_ulong = 8;
    pub const BRDGSTO: libc::c_ulong = 9;
    pub const BRDGGTO: libc::c_ulong = 10;
    pub const BRDGDADDR: libc::c_ulong = 11;
    pub const BRDGFLUSH: libc::c_ulong = 12;
    pub const BRDGGPRI: libc::c_ulong = 13;
    pub const BRDGSPRI: libc::c_ulong = 14;
    pub const BRDGGHT: libc::c_ulong = 15;
    pub const BRDGSHT: libc::c_ulong = 16;
    pub const BRDGGFD: libc::c_ulong = 17;
    pub const BRDGSFD: libc::c_ulong = 18;
    pub const BRDGGMA: libc::c_ulong = 19;
    pub const BRDGSMA: libc::c_ulong = 20;
    pub const BRDGSIFPRIO: libc::c_ulong = 21;
    pub const BRDGSIFCOST: libc::c_ulong = 22;
    pub const BRDGADDS: libc::c_ulong = 23;
    pub const BRDGDELS: libc::c_ulong = 24;
    pub const BRDGPARAM: libc::c_ulong = 25;
    pub const BRDGGRTE: libc::c_ulong = 26;
    pub const BRDGGIFSSTP: libc::c_ulong = 27;
    pub const BRDGSPROTO: libc::c_ulong = 28;
    pub const BRDGSTXHC: libc::c_ulong = 29;
    pub const BRDGSIFAMAX: libc::c_ulong = 30;
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifgrq_ifgrqu {
    pub ifgrqu_group: ::IfName,
    pub ifgrqu_member: ::IfName,
}

impl ifgrq_ifgrqu {
    pub unsafe fn set_group_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifgrqu_group, name)
    }

    pub unsafe fn get_group_name(&self) -> io::Result<String> {
        get_name!(self.ifgrqu_group)
    }

    pub unsafe fn from_group_name(name: &str) -> io::Result<Self> {
        let mut req: Self = mem::zeroed();
        req.set_group_name(name)?;
        Ok(req)
    }

    pub unsafe fn set_member_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifgrqu_member, name)
    }

    pub unsafe fn get_member_name(&self) -> io::Result<String> {
        get_name!(self.ifgrqu_member)
    }

    pub unsafe fn from_member_name(name: &str) -> io::Result<Self> {
        let mut req: Self = mem::zeroed();
        req.set_member_name(name)?;
        Ok(req)
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifg_req {
    pub ifgrq_ifgrqu: ifgrq_ifgrqu,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union ifgr_ifgru {
    pub ifgru_group: ::IfName,
    pub ifgru_groups: *mut ifg_req,
}

impl ifgr_ifgru {
    pub unsafe fn set_group_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifgru_group, name)
    }

    pub unsafe fn get_group_name(&self) -> io::Result<String> {
        get_name!(self.ifgru_group)
    }

    pub unsafe fn from_group_name(name: &str) -> io::Result<Self> {
        let mut req: Self = mem::zeroed();
        req.set_group_name(name)?;
        Ok(req)
    }

    pub unsafe fn get_group_names(&self, len: usize) -> io::Result<Vec<String>> {
        if len % (libc::IFNAMSIZ as usize) != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "bad len"));
        }
        let slice = ::std::slice::from_raw_parts(self.ifgru_groups as *mut u8, len);
        let mut i = libc::IFNAMSIZ as usize;
        let mut v = vec![];
        while i <= len {
            let part = &slice[i - libc::IFNAMSIZ as usize..i];
            let name = get_name!(part)?;
            v.push(name);
            i += libc::IFNAMSIZ
        }
        Ok(v)
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifgroupreq {
    pub ifgr_name: ::IfName,
    pub ifgr_len: libc::c_uint,
    pub ifgr_ifgru: ifgr_ifgru,
}

impl ifgroupreq {
    pub fn set_name(&mut self, name: &str) -> io::Result<()> {
        set_name!(self.ifgr_name, name)
    }

    pub fn get_name(&self) -> io::Result<String> {
        get_name!(self.ifgr_name)
    }

    pub fn from_name(name: &str) -> io::Result<Self> {
        let mut req: Self = unsafe { mem::zeroed() };
        req.set_name(name)?;
        Ok(req)
    }
}

pub const CTL_NET: libc::c_int = 4;
pub const NET_RT_DUMP: libc::c_int = 1;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct rt_metrics {
    pub rmx_locks: libc::c_ulong, /* Kernel must leave these values alone */
    pub rmx_mtu: libc::c_ulong,   /* MTU for this path */
    pub rmx_hopcount: libc::c_ulong, /* max hops expected */
    pub rmx_expire: libc::c_ulong, /* lifetime for route, e.g. redirect */
    pub rmx_recvpipe: libc::c_ulong, /* inbound delay-bandwidth product */
    pub rmx_sendpipe: libc::c_ulong, /* outbound delay-bandwidth product */
    pub rmx_ssthresh: libc::c_ulong, /* outbound gateway buffer limit */
    pub rmx_rtt: libc::c_ulong,   /* estimated round trip time */
    pub rmx_rttvar: libc::c_ulong, /* estimated rtt variance */
    pub rmx_pksent: libc::c_ulong, /* packets sent using this route */
    pub rmx_weight: libc::c_ulong, /* route weight */
    pub rmx_filler: [libc::c_ulong; 3], /* will be used for T/TCP later */
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct rt_msghdr {
    pub rtm_msglen: libc::c_ushort, /* to skip over non-understood messages */
    pub rtm_version: libc::c_uchar, /* future binary compatibility */
    pub rtm_type: libc::c_uchar,    /* message type */
    pub rtm_index: libc::c_ushort,  /* index for associated ifp */
    pub rtm_flags: libc::c_int,     /* flags, incl. kern & message, e.g. DONE */
    pub rtm_addrs: libc::c_int,     /* bitmask identifying sockaddrs in msg */
    pub rtm_pid: libc::pid_t,       /* identify sender */
    pub rtm_seq: libc::c_int,       /* for sender to identify action */
    pub rtm_errno: libc::c_int,     /* why failed */
    pub rtm_fmask: libc::c_int,     /* bitmask used in RTM_CHANGE message */
    pub rtm_inits: libc::c_ulong,   /* which metrics we are initializing */
    pub rtm_rmx: rt_metrics,        /* metrics themselves */
}

pub mod rtm {
    use libc;

    pub const RTM_ADD: libc::c_uchar = 0x1; /* (1) Add Route */
    pub const RTM_DELETE: libc::c_uchar = 0x2; /* (1) Delete Route */
    pub const RTM_CHANGE: libc::c_uchar = 0x3; /* (1) Change Metrics or flags */
    pub const RTM_GET: libc::c_uchar = 0x4; /* (1) Report Metrics */
    pub const RTM_LOSING: libc::c_uchar = 0x5; /* (1) Kernel Suspects Partitioning */
    pub const RTM_REDIRECT: libc::c_uchar = 0x6; /* (1) Told to use different route */
    pub const RTM_MISS: libc::c_uchar = 0x7; /* (1) Lookup failed on this address */
    pub const RTM_LOCK: libc::c_uchar = 0x8; /* (1) fix specified metrics */
    /*	0x9  */
    /*	0xa  */
    pub const RTM_RESOLVE: libc::c_uchar = 0xb; /* (1) req to resolve dst to LL addr */
    pub const RTM_NEWADDR: libc::c_uchar = 0xc; /* (2) address being added to iface */
    pub const RTM_DELADDR: libc::c_uchar = 0xd; /* (2) address being removed from iface */
    pub const RTM_IFINFO: libc::c_uchar = 0xe; /* (3) iface going up/down etc. */
    pub const RTM_NEWMADDR: libc::c_uchar = 0xf; /* (4) mcast group membership being added to if */
    pub const RTM_DELMADDR: libc::c_uchar = 0x10; /* (4) mcast group membership being deleted */
    pub const RTM_IFANNOUNCE: libc::c_uchar = 0x11; /* (5) iface arrival/departure */
    pub const RTM_IEEE80211: libc::c_uchar = 0x12; /* (5) IEEE80211 wireless event */

}

bitflags! {
    // https://github.com/freebsd/freebsd/blob/master/sys/net/route.h
    pub struct RtfFlags: libc::c_int {
        const RTF_UP = 0x1;		/* route usable */
        const RTF_GATEWAY = 0x2;		/* destination is a gateway */
        const RTF_HOST = 0x4;		/* host entry (net otherwise) */
        const RTF_REJECT = 0x8;		/* host or net unreachable */
        const RTF_DYNAMIC = 0x10;		/* created dynamically (by redirect) */
        const RTF_MODIFIED = 0x20;		/* modified dynamically (by redirect) */
        const RTF_DONE = 0x40;		/* message confirmed */
        /*			0x80		   unused, was RTF_DELCLONE */
        /*			0x100		   unused, was RTF_CLONING */
        const RTF_XRESOLVE = 0x200;		/* external daemon resolves name */
        const RTF_LLINFO = 0x400;		/* DEPRECATED - exists ONLY for backward
                            compatibility */
        const RTF_LLDATA = 0x400;		/* used by apps to add/del L2 entries */
        const RTF_STATIC = 0x800;		/* manually added */
        const RTF_BLACKHOLE = 0x1000;		/* just discard pkts (during updates) */
        const RTF_PROTO2 = 0x4000;		/* protocol specific routing flag */
        const RTF_PROTO1 = 0x8000;		/* protocol specific routing flag */
        /*			0x10000		   unused, was RTF_PRCLONING */
        /*			0x20000		   unused, was RTF_WASCLONED */
        const RTF_PROTO3 = 0x40000;		/* protocol specific routing flag */
        const RTF_FIXEDMTU = 0x80000;		/* MTU was explicitly specified */
        const RTF_PINNED = 0x100000;	/* route is immutable */
        const RTF_LOCAL = 0x200000; 	/* route represents a local address */
        const RTF_BROADCAST = 0x400000;	/* route represents a bcast address */
        const RTF_MULTICAST = 0x800000;	/* route represents a mcast address */
                            /* 0x8000000 and up unassigned */
        const RTF_STICKY = 0x10000000;	/* always route dst->src */

        const RTF_RNH_LOCKED = 0x40000000;	/* radix node head is locked */

        const RTF_GWFLAG_COMPAT = 0x80000000;	/* a compatibility bit for interacting
                       with existing routing apps */
    }
}

bitflags! {
    pub struct RtmAddrFlags: i32 {
        const RTA_DST = 0x01;
        const RTA_GATEWAY = 0x02;
        const RTA_NETMASK = 0x04;
        const RTA_GENMASK = 0x08;
        const RTA_IFP = 0x10;
        const RTA_IFA = 0x20;
        const RTA_AUTHOR = 0x40;
        const RTA_BRD = 0x80;
    }
}
