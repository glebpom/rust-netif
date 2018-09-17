use ifstructs::{
    self, brcmd, ifaliasreq, ifbreq, ifdrv, ifgroupreq, ifreq, rt_msghdr, IfName, RtfFlags,
    RtmAddrFlags,
};
use libc;
use nix;
use nix::sys::socket::SockAddr;
use std::ffi::CString;
use std::io::Seek;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::{io, ptr};

// #define	SIOCAIFADDR	 _IOW('i', 43, struct ifaliasreq)/* add/chg IF alias */
ioctl_write_ptr!(iface_add_addr, b'i', 43, ifaliasreq);

// #define	SIOCIFCREATE	_IOWR('i', 122, struct ifreq)	/* create clone if */
ioctl_readwrite!(ioctl_create_clone_iface, b'i', 122, ifreq);
// #define	SIOCSIFNAME	 _IOW('i', 40, struct ifreq)	/* set IF name */
ioctl_write_ptr!(ioctl_set_iface_name, b'i', 40, ifreq);
// #define	SIOCSDRVSPEC	_IOW('i', 123, struct ifdrv)	/* set driver-specific parameters */
ioctl_write_ptr!(ioctl_iface_drvspec, b'i', 123, ifdrv);
// #define	SIOCIFDESTROY	 _IOW('i', 121, struct ifreq)	/* destroy clone if */
ioctl_write_ptr!(ioctl_iface_destroy, b'i', 121, ifreq);
// #define	SIOCGIFGROUP	_IOWR('i', 136, struct ifgroupreq) /* get ifgroups */
ioctl_readwrite!(ioctl_iface_get_groups, b'i', 136, ifgroupreq);
// #define	SIOCGIFINDEX	_IOWR('i', 32, struct ifreq)	/* get IF index */
ioctl_write_ptr!(ioctl_get_iface_index, b'i', 32, ifreq);

pub fn get_iface_groups<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<Vec<String>, IfError> {
    let mut req = ifgroupreq::from_name(&ifname)?;
    unsafe { ioctl_iface_get_groups(ctl_fd.as_raw_fd(), &mut req) }?;
    let required_len = req.ifgr_len as usize;

    let mut ifgru_groups = vec![0u8; required_len];

    req.ifgr_ifgru.ifgru_groups = ifgru_groups.as_mut_ptr() as *mut _ as *mut _;
    unsafe { ioctl_iface_get_groups(ctl_fd.as_raw_fd(), &mut req) }?;

    Ok(unsafe { req.ifgr_ifgru.get_group_names(required_len)? })
}

pub fn create_bridge<F: AsRawFd>(ctl_fd: &F, bridge_ifname: &str) -> Result<(), IfError> {
    let mut req = ifreq::from_name("bridge")?;
    unsafe { ioctl_create_clone_iface(ctl_fd.as_raw_fd(), &mut req)? };

    req.ifr_ifru.ifru_data = CString::new(bridge_ifname).unwrap().into_raw();

    let res = unsafe { ioctl_set_iface_name(ctl_fd.as_raw_fd(), &mut req) };

    unsafe { CString::from_raw(req.ifr_ifru.ifru_data) };

    res?;

    Ok(())
}

pub fn remove_bridge<F: AsRawFd>(ctl_fd: &F, bridge_ifname: &str) -> Result<(), IfError> {
    let mut req = ifreq::from_name(bridge_ifname)?;
    unsafe { ioctl_iface_destroy(ctl_fd.as_raw_fd(), &mut req) }?;
    Ok(())
}

pub fn add_iface_to_bridge<F: AsRawFd>(
    ctl_fd: &F,
    bridge_ifname: &str,
    iface_ifname: &str,
) -> Result<(), IfError> {
    let mut ifd = ifdrv::from_name(bridge_ifname)?;
    let mut b_req = ifbreq::from_name(iface_ifname)?;

    ifd.ifd_len = mem::size_of::<ifbreq>();
    ifd.ifd_data = &mut b_req as *mut _ as *mut _;

    // https://github.com/freebsd/freebsd/blob/406cc909dab1b86d97162cce12954ba444cc9e6a/usr.sbin/bsnmpd/modules/snmp_bridge/bridge_sys.c#L1003
    ifd.ifd_cmd = brcmd::BRDGADD;

    unsafe { ioctl_iface_drvspec(ctl_fd.as_raw_fd(), &mut ifd) }?;

    Ok(())
}

pub fn remove_iface_from_bridge<F: AsRawFd>(
    ctl_fd: &F,
    bridge_ifname: &str,
    iface_ifname: &str,
) -> Result<(), IfError> {
    let mut ifd = ifdrv::from_name(bridge_ifname)?;
    let mut b_req = ifbreq::from_name(iface_ifname)?;

    ifd.ifd_len = mem::size_of::<ifbreq>();
    ifd.ifd_data = &mut b_req as *mut _ as *mut _;

    ifd.ifd_cmd = brcmd::BRDGDEL;

    unsafe { ioctl_iface_drvspec(ctl_fd.as_raw_fd(), &mut ifd) }?;

    Ok(())
}

#[derive(Clone, Debug)]
pub struct RouteRecord {
    destination: Option<SockAddr>,
    gateway: Option<SockAddr>,
    netmask: Option<SockAddr>,
    iface_addr: Option<SockAddr>,
    iface: ::Iface,
    flags: RtfFlags,
}

pub fn read_sockaddr_if_flag(
    i: &mut usize,
    buf: &[u8],
    flags: RtmAddrFlags,
    if_flag: RtmAddrFlags,
) -> Option<SockAddr> {
    if flags.contains(if_flag) {
        let s = mem::size_of::<libc::sockaddr>();
        *i = *i + s;
        unsafe {
            SockAddr::from_libc_sockaddr(&ptr::read::<libc::sockaddr>(
                &buf[*i - s..*i] as *const _ as *const _,
            ))
        }
    } else {
        None
    }
}

pub fn list_routes<F: AsRawFd>(ctl_fd: &F) -> Result<Vec<RouteRecord>, IfError> {
    let family = 0;
    let flags = 0;
    let mut lenp: usize = 0;

    let mib: [i32; 6] = [
        ifstructs::CTL_NET,
        libc::AF_ROUTE,
        0,
        family,
        ifstructs::NET_RT_DUMP,
        flags,
    ];

    let null: *const i32 = ptr::null();
    let mut ret: libc::c_int = 0;

    ret = unsafe {
        libc::sysctl(
            mib.as_ptr() as *mut i32,
            6,
            null as *mut libc::c_void,
            &mut lenp,
            null as *mut libc::c_void,
            0,
        )
    };

    if ret < 0 {
        return Err(nix::Error::last().into());
    }

    let buf = vec![0u8; lenp];
    let buf_ptr = buf.as_ptr();

    ret = unsafe {
        libc::sysctl(
            mib.as_ptr() as *mut i32,
            6,
            buf_ptr as *mut libc::c_void,
            &mut lenp,
            null as *mut libc::c_void,
            0,
        )
    };
    if ret < 0 {
        return Err(nix::Error::last().into());
    }

    let mut routing_table = vec![];

    let rt_msghdr_size = mem::size_of::<rt_msghdr>();
    let sockaddr_size = mem::size_of::<libc::sockaddr>();

    let mut i = 0;
    while i < lenp {
        let mut msg_start_idx = i;
        let hdr =
            unsafe { ptr::read::<rt_msghdr>(&buf[i..i + rt_msghdr_size] as *const _ as *const _) };
        i += rt_msghdr_size;

        let rtm_flags = RtfFlags::from_bits(hdr.rtm_flags).unwrap();
        let rtm_addr_flags = RtmAddrFlags::from_bits(hdr.rtm_addrs).unwrap();

        let destination =
            read_sockaddr_if_flag(&mut i, &buf, rtm_addr_flags, RtmAddrFlags::RTA_DST);
        let gateway =
            read_sockaddr_if_flag(&mut i, &buf, rtm_addr_flags, RtmAddrFlags::RTA_GATEWAY);
        let netmask =
            read_sockaddr_if_flag(&mut i, &buf, rtm_addr_flags, RtmAddrFlags::RTA_NETMASK);
        let _iface_name =
            read_sockaddr_if_flag(&mut i, &buf, rtm_addr_flags, RtmAddrFlags::RTA_IFP);
        let iface_addr = read_sockaddr_if_flag(&mut i, &buf, rtm_addr_flags, RtmAddrFlags::RTA_IFA);
        let author = read_sockaddr_if_flag(&mut i, &buf, rtm_addr_flags, RtmAddrFlags::RTA_AUTHOR);
        let brd = read_sockaddr_if_flag(&mut i, &buf, rtm_addr_flags, RtmAddrFlags::RTA_BRD);

        let unprocessed_bytes = (hdr.rtm_msglen as usize) - (i - msg_start_idx);
        if unprocessed_bytes > 0 {
            i += unprocessed_bytes;
        }

        routing_table.push(RouteRecord {
            destination,
            gateway,
            netmask,
            iface_addr,
            iface: ::Iface::find_by_name(&get_iface_name(ctl_fd, hdr.rtm_index as libc::c_short)?)?,
            flags: rtm_flags,
        });
    }

    Ok(routing_table)
}

pub fn get_iface_name<F: AsRawFd>(ctl_fd: &F, idx: libc::c_short) -> Result<String, IfError> {
    let mut ifname: IfName = unsafe { mem::zeroed() };
    let res = unsafe { if_indextoname(idx as libc::c_uint, ifname.as_mut_ptr() as *mut _) };
    if res.is_null() {
        return Err(nix::Error::last().into());
    }
    let res = get_name!(ifname);
    Ok(res?)
}

pub fn get_iface_index<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<libc::c_short, IfError> {
    let mut req = ifreq::from_name(ifname)?;
    unsafe { ::impls::ioctl_get_iface_index(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(unsafe { req.ifr_ifru.ifru_index }.into())
}

extern "C" {
    fn if_indextoname(ifindex: libc::c_uint, ifname: *mut libc::c_char) -> *mut libc::c_char;
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_bridge() {
    //     let ctl_fd = ::impls::new_control_socket().unwrap();
    //     create_bridge(&ctl_fd, "lcstrbr0").expect("create bridge lcstrbr0");
    //     get_iface_groups(&ctl_fd, "lcstrbr0").expect("get groups");
    //     ::Iface::find_by_name("lcstrbr0")
    //         .unwrap()
    //         .up()
    //         .expect("up bridge lcstrbr0");
    //     add_iface_to_bridge(&ctl_fd, "lcstrbr0", "tap1").expect("add tap1 to bridge lcstrbr0");
    //     remove_iface_from_bridge(&ctl_fd, "lcstrbr0", "tap1")
    //         .expect("remove tap1 from bridge lcstrbr0");
    //     ::Iface::find_by_name("lcstrbr0")
    //         .unwrap()
    //         .down()
    //         .expect("down bridge lcstrbr0");
    //     remove_bridge(&ctl_fd, "lcstrbr0").expect("remove bridge lcstrbr0");
    // }

    #[test]
    fn test_routes() {
        let ctl_fd = ::impls::new_control_socket().unwrap();
        let routes = list_routes(&ctl_fd).expect("routes!");
        println!("routes = {:?}", routes);
    }
}
