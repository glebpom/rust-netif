use ifstructs::ifreq;
use ifstructs::IfFlags;
use libc;
use nix;
use nix::sys::socket::{socket, AddressFamily, InetAddr, SockAddr, SockFlag, SockType};
use std::fs::File;
use std::net::{IpAddr, SocketAddr};
use std::os::unix::io::{AsRawFd, FromRawFd};
use crate::IfError;

macro_rules! ti {
    ($e:expr) => {
        match $e {
            Ok(r) => Ok(r),
            Err(nix::Error::Sys(nix::errno::Errno::ENXIO)) => {
                Err(crate::IfError::NotFound)
            }
            Err(e) => Err(crate::IfError::from(e)),
        }
    };
}

pub fn new_control_socket() -> Result<File, IfError> {
    Ok(unsafe {
        File::from_raw_fd(socket(
            AddressFamily::Inet,
            SockType::Datagram,
            SockFlag::empty(),
            None,
        )?)
    })
}

pub fn get_iface_ifreq<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<ifreq, IfError> {
    let mut req = ifreq::from_name(ifname)?;
    ti!(unsafe { iface_get_flags(ctl_fd.as_raw_fd(), &mut req) })?;
    Ok(req)
}

pub fn is_up<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<bool, IfError> {
    let mut req = ifreq::from_name(ifname)?;
    ti!(unsafe { iface_get_flags(ctl_fd.as_raw_fd(), &mut req) })?;

    let flags = unsafe { req.get_flags() };

    Ok(flags.contains(IfFlags::IFF_UP) && flags.contains(IfFlags::IFF_RUNNING))
}

pub fn set_promiscuous_mode<F: AsRawFd>(ctl_fd: &F, ifname: &str, is_enable: bool) -> Result<(), IfError> {
    let mut req = ifreq::from_name(ifname)?;
    ti!(unsafe { iface_get_flags(ctl_fd.as_raw_fd(), &mut req) })?;

    let mut flags = unsafe { req.get_flags() };

    flags.set(IfFlags::IFF_PROMISC, is_enable);

    unsafe { req.set_flags(flags) };

    unsafe { iface_set_flags(ctl_fd.as_raw_fd(), &mut req) }?;

    Ok(())
}

pub fn up<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<(), IfError> {
    if is_up(ctl_fd, ifname)? {
        return Ok(());
    }

    let mut req = ifreq::from_name(ifname)?;
    ti!(unsafe { iface_get_flags(ctl_fd.as_raw_fd(), &mut req) })?;

    let mut flags = unsafe { req.get_flags() };

    flags.insert(IfFlags::IFF_UP);
    flags.insert(IfFlags::IFF_RUNNING);

    unsafe { req.set_flags(flags) };

    unsafe { iface_set_flags(ctl_fd.as_raw_fd(), &mut req) }?;

    Ok(())
}

pub fn down<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<(), IfError> {
    if !is_up(ctl_fd, ifname)? {
        return Ok(());
    }

    let mut req = ifreq::from_name(ifname)?;
    ti!(unsafe { iface_get_flags(ctl_fd.as_raw_fd(), &mut req) })?;

    let mut flags = unsafe { req.get_flags() };

    flags.remove(IfFlags::IFF_UP);
    flags.remove(IfFlags::IFF_RUNNING);

    unsafe { req.set_flags(flags) };

    unsafe { iface_set_flags(ctl_fd.as_raw_fd(), &mut req) }?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn get_all_addresses() -> Result<nix::ifaddrs::InterfaceAddressIterator, IfError> {
    Ok(nix::ifaddrs::getifaddrs()?)
}

pub fn to_sockaddr(ip: IpAddr) -> libc::sockaddr {
    let r = SockAddr::new_inet(InetAddr::from_std(&SocketAddr::new(ip, 0)));
    let f = unsafe { r.as_ffi_pair() };
    let mut res = f.0.clone();
    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    {
        res.sa_len = f.1 as u8;
    }
    res
}

cfg_if! {
    if #[cfg(any(target_os = "linux",
                 target_os = "android",
                 target_os = "fuchsia"))] {
        mod notbsd;
        pub use self::notbsd::*;
    } else if #[cfg(any(target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "dragonfly",
                        target_os = "openbsd",
                        target_os = "netbsd",
                        target_os = "bitrig"))] {
        mod bsd;
        pub use self::bsd::*;
    } else {
        // Unknown target_os
    }
}
