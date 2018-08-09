extern crate eui48;
#[cfg(unix)]
extern crate ifstructs;
extern crate libc;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate nix;
#[macro_use]
extern crate error_chain;
extern crate ipnetwork;

mod errors;
mod impls;

pub use errors::{Error, ErrorKind, Result};

use eui48::MacAddress;
#[cfg(unix)]
use ifstructs::IfFlags;
use std::net::IpAddr;

pub trait IfaceType: ::std::fmt::Debug {
    fn hw_addr(&self) -> Option<MacAddress>;
}

#[derive(Debug)]
pub struct Ethernet {
    hw_addr: MacAddress,
}
impl IfaceType for Ethernet {
    fn hw_addr(&self) -> Option<MacAddress> {
        Some(self.hw_addr)
    }
}

#[derive(Debug)]
pub struct Bridge {
    hw_addr: MacAddress,
}
impl IfaceType for Bridge {
    fn hw_addr(&self) -> Option<MacAddress> {
        Some(self.hw_addr)
    }
}

#[derive(Debug)]
pub struct Loopback {}
impl IfaceType for Loopback {
    fn hw_addr(&self) -> Option<MacAddress> {
        None
    }
}

#[derive(Debug)]
pub struct Unknown {
    hw_addr: Option<MacAddress>,
}
impl IfaceType for Unknown {
    fn hw_addr(&self) -> Option<MacAddress> {
        self.hw_addr
    }
}

#[derive(Debug)]
pub struct Iface {
    ifname: String,
    ip_addrs: Vec<IpAddr>,
    flags: IfFlags,
    details: Box<IfaceType>,
}

#[cfg(unix)]
impl Iface {
    #[cfg(all(unix, not(target_os = "android")))]
    pub fn all() -> Result<Vec<Iface>> {
        use std::collections::HashMap;

        let mut hw = HashMap::new();
        let mut ips = HashMap::new();
        let mut flags = HashMap::new();
        for ifaddr in impls::get_all_addresses()? {
            flags.insert(ifaddr.interface_name.clone(), ifaddr.flags);
            if let Some(addr) = ifaddr.address {
                match addr {
                    nix::sys::socket::SockAddr::Inet(inet) => {
                        let ip_addrs_entry = ips.entry(ifaddr.interface_name).or_insert(vec![]);
                        ip_addrs_entry.push(inet);
                    }
                    nix::sys::socket::SockAddr::Link(link) => {
                        hw.insert(ifaddr.interface_name, link);
                    }
                    _ => {}
                }
            }
        }
        let mut ifaces = Vec::new();
        for (ifname, flags) in flags {
            let ip_addrs = ips.get(&ifname)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|inet_addr| inet_addr.to_std().ip())
                .collect();
            let hw_addr = hw.get(&ifname).map(|addr| MacAddress::new(addr.addr()));
            let details: Box<dyn IfaceType> = if flags.contains(IfFlags::IFF_LOOPBACK) {
                Box::new(Loopback {})
            } else if let Some(hw_addr) = hw_addr {
                Box::new(Ethernet {
                    hw_addr,
                })
            } else {
                Box::new(Unknown {
                    hw_addr,
                })
            };

            let iface = Iface {
                ip_addrs,
                ifname,
                details,
                flags,
            };
            ifaces.push(iface);
        }
        Ok(ifaces)
    }

    #[cfg(all(unix, not(target_os = "android")))]
    pub fn find_by_name(ifname: &str) -> Result<Iface> {
        let iface = Self::all()?
            .into_iter()
            .find(|x| x.ifname == ifname)
            .ok_or(Error::from(ErrorKind::IfaceNotFound))?;
        let ctl_fd = impls::new_control_socket()?;
        impls::get_iface_ifreq(&ctl_fd, &iface.ifname)?;
        Ok(iface)
    }

    #[cfg(target_os = "android")]
    pub fn find_by_name(ifname: &str) -> Result<Iface> {
        Err(ErrorKind::IfaceNotFound.into())
    }

    pub fn hw_addr(&self) -> Option<MacAddress> {
        self.details.hw_addr()
    }

    pub fn is_up(&self) -> Result<bool> {
        let ctl_fd = impls::new_control_socket()?;
        impls::is_up(&ctl_fd, &self.ifname)
    }

    pub fn up(&self) -> Result<()> {
        let ctl_fd = impls::new_control_socket()?;
        impls::up(&ctl_fd, &self.ifname)
    }

    pub fn down(&self) -> Result<()> {
        let ctl_fd = impls::new_control_socket()?;
        impls::down(&ctl_fd, &self.ifname)
    }

    pub fn set_promiscuous_mode(&self, is_enable: bool) -> Result<()> {
        let ctl_fd = impls::new_control_socket()?;
        impls::set_promiscuous_mode(&ctl_fd, &self.ifname, is_enable)
    }

    pub fn refresh(&mut self) -> Result<()> {
        let new_iface = Self::find_by_name(&self.ifname)?;
        self.ip_addrs = new_iface.ip_addrs;
        self.flags = new_iface.flags;
        self.details = new_iface.details;
        Ok(())
    }

    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    pub fn add_addr(&mut self, cidr: ipnetwork::IpNetwork) -> Result<()> {
        let ctl_fd = impls::new_control_socket()?;

        impls::add_addr_to_iface(
            &ctl_fd,
            &self.ifname,
            cidr.ip(),
            cidr.mask(),
            cidr.broadcast(),
        )?;

        self.refresh()?;

        Ok(())
    }

    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    pub fn del_addr(&mut self, ip: IpAddr) -> Result<()> {
        let ctl_fd = impls::new_control_socket()?;

        impls::del_addr_from_iface(&ctl_fd, &self.ifname, ip)?;

        self.refresh()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    const default_iface: &str = "lo0";
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    const default_iface: &str = "lo";

    #[test]
    fn test_not_found() {
        match Iface::find_by_name("not_exist") {
            Err(Error(ErrorKind::IfaceNotFound, _)) => {}
            _ => panic!("bad error type"),
        }
    }

    #[test]
    fn test_found() {
        match Iface::find_by_name(default_iface) {
            Ok(_) => {}
            _ => panic!("bad error type"),
        }
    }

    #[test]
    fn test_is_up() {
        let iface = Iface::find_by_name(default_iface).unwrap();
        assert!(iface.is_up().unwrap());
    }

    #[test]
    fn test_up() {
        let iface = Iface::find_by_name(default_iface).unwrap();
        iface.up().unwrap();
    }

    // #[test]
    // fn test_down() {
    //     let iface = Iface::find_by_name(default_iface).unwrap();
    //     iface.down().unwrap();
    // }

    // #[test]
    // fn test_promiscuous() {
    //     let iface = Iface::find_by_name(default_iface).unwrap();
    //     iface.set_promiscuous_mode(true).unwrap();
    // }

    #[test]
    fn test_list_all() {
        Iface::all().unwrap();
    }
}
