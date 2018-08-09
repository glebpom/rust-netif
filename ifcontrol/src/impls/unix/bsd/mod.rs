
use errors::Result;
use ifstructs::{ifaliasreq, ifreq};
use impls::unix::to_sockaddr;
use std::net::IpAddr;
use std::os::unix::io::AsRawFd;

// #define	SIOCGIFFLAGS	_IOWR('i', 17, struct ifreq)	/* get ifnet flags */
ioctl_readwrite!(iface_get_flags, b'i', 17, ifreq);
// #define	SIOCSIFFLAGS	 _IOW('i', 16, struct ifreq)	/* set ifnet flags */
ioctl_write_ptr!(iface_set_flags, b'i', 16, ifreq);

//#define	SIOCAIFADDR	 _IOW('i', 26, struct ifaliasreq)/* add/chg IF alias */
#[cfg(not(target_os = "freebsd"))]
ioctl_write_ptr!(iface_add_addr, b'i', 26, ifaliasreq);

// #define	SIOCDIFADDR	 _IOW('i', 25, struct ifreq)	/* delete IF addr */
ioctl_write_ptr!(iface_del_addr, b'i', 25, ifreq);

pub fn add_addr_to_iface<F: AsRawFd>(
    ctl_fd: &F,
    ifname: &str,
    ip: IpAddr,
    mask: IpAddr,
    broadcast_addr: IpAddr,
) -> Result<()> {
    let mut req = ifaliasreq::from_name(ifname)?;

    req.ifra_addr = to_sockaddr(ip);
    req.ifra_broadaddr = to_sockaddr(broadcast_addr);
    req.ifra_mask = to_sockaddr(mask);

    unsafe { iface_add_addr(ctl_fd.as_raw_fd(), &mut req) }?;

    Ok(())
}

pub fn del_addr_from_iface<F: AsRawFd>(ctl_fd: &F, ifname: &str, ip: IpAddr) -> Result<()> {
    let mut req = ifreq::from_name(ifname)?;

    unsafe { req.set_addr(to_sockaddr(ip)) };

    unsafe { iface_del_addr(ctl_fd.as_raw_fd(), &mut req) }?;

    Ok(())
}

cfg_if! {
    if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        mod apple;
        pub use self::apple::*;
    } else if #[cfg(any(target_os = "openbsd", target_os = "netbsd",
                        target_os = "bitrig"))] {
        mod netbsdlike;
        pub use self::netbsdlike::*;
    } else if #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))] {
        mod freebsdlike;
        pub use self::freebsdlike::*;
    } else {
        // Unknown target_os
    }
}
