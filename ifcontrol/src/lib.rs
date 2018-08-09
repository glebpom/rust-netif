extern crate eui48;
extern crate ifstructs;
extern crate libc;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate nix;
#[macro_use]
extern crate error_chain;

mod errors;
mod impls;

pub use errors::{Error, ErrorKind, Result};

use eui48::MacAddress;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct Iface {
    ifname: String,
    hw_addr: Option<MacAddress>,
    ip_addrs: Vec<IpAddr>,
    //TODO: add flags
}

impl Iface {
    #[cfg(not(target_os = "android"))]
    pub fn all() -> Result<Vec<Iface>> {
        let mut ifnames = HashSet::new();
        let mut hw = HashMap::new();
        let mut ips = HashMap::new();
        for ifaddr in impls::get_all_addresses()? {
            ifnames.insert(ifaddr.interface_name.clone());
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
        let mut ifaces = vec![];
        for iface_name in ifnames {
            let iface = Iface {
                hw_addr: hw.get(&iface_name).map(|addr| MacAddress::new(addr.addr())),
                ip_addrs: ips.get(&iface_name)
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|inet_addr| inet_addr.to_std().ip())
                    .collect(),
                ifname: iface_name,
            };
            ifaces.push(iface);
        }
        Ok(ifaces)
    }

    #[cfg(not(target_os = "android"))]
    pub fn find_by_name(ifname: &str) -> Result<Iface> {
        Iface::all()?
            .iter()
            .find(|&x| x.ifname == ifname)
            .cloned()
            .ok_or(ErrorKind::IfaceNotFound.into())
    }

    #[cfg(target_os = "android")]
    pub fn find_by_name(ifname: &str) -> Result<Iface> {
        let ctl_fd = impls::new_control_socket()?;
        impls::get_iface_ifreq(&ctl_fd, ifname)?;
        Ok(Iface {
            ifname: ifname.to_owned(),
            hw_addr: None,
            ip_addrs: vec![],
        })
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

    pub fn hw_addr(&self) -> Option<MacAddress> {
        self.hw_addr
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

    #[test]
    fn test_list_all() {
        Iface::all().unwrap();
    }

}
