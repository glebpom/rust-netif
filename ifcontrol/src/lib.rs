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
extern crate failure;
extern crate ipnetwork;

mod impls;

#[derive(Debug, Fail)]
#[fail(display = "interface control error")]
pub enum IfError {
    #[cfg(unix)]
    #[fail(display = "nix error: {}", _0)]
    Nix(#[cause] ::nix::Error),
    #[fail(display = "io error: {}", _0)]
    Io(#[cause] ::std::io::Error),
    #[fail(display = "iface not found")]
    NotFound,
}

#[cfg(unix)]
impl From<::nix::Error> for IfError {
    fn from(e: ::nix::Error) -> IfError {
        IfError::Nix(e)
    }
}

impl From<::std::io::Error> for IfError {
    fn from(e: ::std::io::Error) -> IfError {
        IfError::Io(e)
    }
}

use eui48::MacAddress;
#[cfg(unix)]
use ifstructs::IfFlags;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub enum Link {
    Regular,
    Tun,
    Tap,
    Bridge,
    Loopback,
    Ethernet,
}

pub type AddressIterator<'a> = std::iter::Cloned<std::slice::Iter<'a, IpAddr>>;

#[cfg(unix)]
#[derive(Debug, Clone)]
pub struct Iface {
    ifname: String,
    ip_addrs: Vec<IpAddr>,
    hw_addr: Option<MacAddress>,
    flags: IfFlags,
    link: Link,
}

#[cfg(unix)]
impl Iface {
    #[cfg(all(unix, not(target_os = "android")))]
    pub fn all() -> Result<Vec<Iface>, IfError> {
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
            let ip_addrs = ips
                .get(&ifname)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|inet_addr| inet_addr.to_std().ip())
                .collect();
            let hw_addr = hw
                .get(&ifname)
                .and_then(|hw| {
                    if hw.addr() == [0, 0, 0, 0, 0, 0] {
                        None
                    } else {
                        Some(hw)
                    }
                })
                .map(|addr| MacAddress::new(addr.addr()));
            let link = if flags.contains(IfFlags::IFF_LOOPBACK) {
                Link::Loopback
            } else if Self::is_bridge(&ifname)? {
                Link::Bridge
            } else if Self::is_tap(&ifname)? {
                Link::Tap
            } else if Self::is_tun(&ifname)? {
                Link::Tun
            } else {
                Link::Ethernet
            };

            let iface = Iface {
                hw_addr,
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
    pub fn find_by_name(ifname: &str) -> Result<Iface, IfError> {
        let iface = Self::all()?
            .into_iter()
            .find(|x| x.ifname == ifname)
            .ok_or(IfError::NotFound)?;
        let ctl_fd = impls::new_control_socket()?;
        impls::get_iface_ifreq(&ctl_fd, &iface.ifname)?;
        Ok(iface)
    }

    #[cfg(target_os = "android")]
    pub fn find_by_name(ifname: &str) -> Result<Iface, IfError> {
        Err(IfError::NotFound)
    }

    pub fn hw_addr(&self) -> Option<MacAddress> {
        self.hw_addr
    }

    pub fn is_up(&self) -> Result<bool, IfError> {
        let ctl_fd = impls::new_control_socket()?;
        impls::is_up(&ctl_fd, &self.ifname)
    }

    pub fn up(&self) -> Result<(), IfError> {
        Iface::up_by_name(&self.ifname)
    }

    pub fn up_by_name(ifname: &str) -> Result<(), IfError> {
        let ctl_fd = impls::new_control_socket()?;
        impls::up(&ctl_fd, ifname)
    }

    pub fn down(&self) -> Result<(), IfError> {
        Iface::down_by_name(&self.ifname)
    }

    pub fn down_by_name(ifname: &str) -> Result<(), IfError> {
        let ctl_fd = impls::new_control_socket()?;
        impls::down(&ctl_fd, ifname)
    }

    pub fn set_promiscuous_mode(&self, is_enable: bool) -> Result<(), IfError> {
        let ctl_fd = impls::new_control_socket()?;
        impls::set_promiscuous_mode(&ctl_fd, &self.ifname, is_enable)
    }

    pub fn refresh(&mut self) -> Result<(), IfError> {
        let new_iface = Self::find_by_name(&self.ifname)?;
        self.ip_addrs = new_iface.ip_addrs;
        self.flags = new_iface.flags;
        self.link = new_iface.link;
        Ok(())
    }

    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    pub fn add_addr(&mut self, cidr: ipnetwork::IpNetwork) -> Result<(), IfError> {
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
    pub fn del_addr(&mut self, ip: IpAddr) -> Result<(), IfError> {
        let ctl_fd = impls::new_control_socket()?;

        impls::del_addr_from_iface(&ctl_fd, &self.ifname, ip)?;

        self.refresh()?;

        Ok(())
    }

    pub fn ip_addresses(&self) -> AddressIterator {
        self.ip_addrs.iter().cloned()
    }
}

#[cfg(all(unix, not(any(target_os = "freebsd", target_os = "linux"))))]
impl Iface {
    fn is_bridge(_ifname: &str) -> Result<bool, IfError> {
        Ok(false)
    }
    fn is_tun(_ifname: &str) -> Result<bool, IfError> {
        Ok(false)
    }
    fn is_tap(_ifname: &str) -> Result<bool, IfError> {
        Ok(false)
    }
}
#[cfg(target_os = "linux")]
impl Iface {
    fn is_bridge(ifname: &str) -> Result<bool, IfError> {
        let ctl_fd = impls::new_control_socket()?;
        let drv = impls::get_ethernet_driver(&ctl_fd, ifname)?;
        Ok(drv.driver == "bridge")
    }
    fn is_tun(ifname: &str) -> Result<bool, IfError> {
        let ctl_fd = impls::new_control_socket()?;
        let drv = impls::get_ethernet_driver(&ctl_fd, ifname)?;
        Ok(drv.driver == "tun" && drv.bus_info == "tun")
    }
    fn is_tap(ifname: &str) -> Result<bool, IfError> {
        let ctl_fd = impls::new_control_socket()?;
        let drv = impls::get_ethernet_driver(&ctl_fd, ifname)?;
        Ok(drv.driver == "tun" && drv.bus_info == "tap")
    }

    pub fn bind_to_device<S: std::os::unix::io::AsRawFd>(
        &mut self,
        socket: &S,
    ) -> Result<(), IfError> {
        Ok(impls::bind_to_device(socket, &self.ifname)?)
    }
}

#[cfg(target_os = "freebsd")]
impl Iface {
    fn has_group(ifname: &str, group: &str) -> Result<bool, IfError> {
        let ctl_fd = impls::new_control_socket()?;
        Ok(impls::get_iface_groups(&ctl_fd, ifname)?
            .iter()
            .find(|s| *s == group)
            .is_some())
    }

    fn is_bridge(ifname: &str) -> Result<bool, IfError> {
        Self::has_group(ifname, "bridge")
    }
    fn is_tun(ifname: &str) -> Result<bool, IfError> {
        Self::has_group(ifname, "tun")
    }
    fn is_tap(ifname: &str) -> Result<bool, IfError> {
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
            Err(IfError::NotFound) => {}
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
