extern crate eui48;
#[cfg(unix)]
#[macro_use]
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

#[derive(Debug, Clone)]
pub enum Link {
    Ethernet(Ethernet),
    Loopback,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Ethernet {
    hw_addr: MacAddress,
    ether_type: EthernetDevice,
}

#[derive(Debug, Clone)]
pub enum EthernetDevice {
    Regular,
    Tun,
    Tap,
    Bridge,
}

#[cfg(unix)]
#[derive(Debug, Clone)]
pub struct Iface {
    ifname: String,
    ip_addrs: Vec<IpAddr>,
    flags: IfFlags,
    link: Link,
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
            let link = if flags.contains(IfFlags::IFF_LOOPBACK) {
                Link::Loopback
            } else if let Some(hw_addr) = hw_addr {
                Link::Ethernet(Ethernet {
                    hw_addr: hw_addr,
                    ether_type: if Self::is_bridge(&ifname)? {
                        EthernetDevice::Bridge
                    } else if Self::is_tap(&ifname)? {
                        EthernetDevice::Tap
                    } else if Self::is_tun(&ifname)? {
                        EthernetDevice::Tun
                    } else {
                        EthernetDevice::Regular
                    },
                })
            } else {
                Link::Unknown
            };

            let iface = Iface {
                ip_addrs,
                ifname,
                flags,
                link,
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
        match self.link {
            Link::Ethernet(Ethernet { hw_addr, .. }) => Some(hw_addr),
            _ => None,
        }
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
        self.link = new_iface.link;
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

#[cfg(all(unix, not(any(target_os = "freebsd", target_os = "linux"))))]
impl Iface {
    fn is_bridge(_ifname: &str) -> Result<bool> {
        Ok(false)
    }
    fn is_tun(_ifname: &str) -> Result<bool> {
        Ok(false)
    }
    fn is_tap(_ifname: &str) -> Result<bool> {
        Ok(false)
    }
}
#[cfg(target_os = "linux")]
impl Iface {
    fn is_bridge(ifname: &str) -> Result<bool> {
        Ok(false)
    }
    fn is_tun(ifname: &str) -> Result<bool> {
        Ok(false)
    }
    fn is_tap(ifname: &str) -> Result<bool> {
        Ok(false)
    }

    pub fn bind_to_device<S: std::os::unix::io::AsRawFd>(&mut self, socket: &S) -> Result<()> {
        Ok(impls::bind_to_device(socket, &self.ifname)?)
    }
}

#[cfg(target_os = "freebsd")]
impl Iface {
    fn has_group(ifname: &str, group: &str) -> Result<bool> {
        let ctl_fd = impls::new_control_socket()?;
        Ok(impls::get_iface_groups(&ctl_fd, ifname)?
            .iter()
            .find(|s| *s == group)
            .is_some())
    }

    fn is_bridge(ifname: &str) -> Result<bool> {
        Self::has_group(ifname, "bridge")
    }
    fn is_tun(ifname: &str) -> Result<bool> {
        Self::has_group(ifname, "tun")
    }
    fn is_tap(ifname: &str) -> Result<bool> {
        Self::has_group(ifname, "tap")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    const DEFAULT_IFACE: &str = "lo0";
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    const DEFAULT_IFACE: &str = "lo";

    #[test]
    fn test_not_found() {
        match Iface::find_by_name("not_exist") {
            Err(Error(ErrorKind::IfaceNotFound, _)) => {}
            _ => panic!("bad error type"),
        }
    }

    #[test]
    fn test_found() {
        match Iface::find_by_name(DEFAULT_IFACE) {
            Ok(_) => {}
            _ => panic!("bad error type"),
        }
    }

    #[test]
    fn test_is_up() {
        let iface = Iface::find_by_name(DEFAULT_IFACE).unwrap();
        assert!(iface.is_up().unwrap());
    }

    #[test]
    fn test_up() {
        let iface = Iface::find_by_name(DEFAULT_IFACE).unwrap();
        iface.up().unwrap();
    }

    // #[test]
    // fn test_down() {
    //     let iface = Iface::find_by_name(DEFAULT_IFACE).unwrap();
    //     iface.down().unwrap();
    // }

    // #[test]
    // fn test_promiscuous() {
    //     let iface = Iface::find_by_name(DEFAULT_IFACE).unwrap();
    //     iface.set_promiscuous_mode(true).unwrap();
    // }

    #[test]
    fn test_list_all() {
        println!("Ifaces: {:?}", Iface::all().unwrap());
    }
}
