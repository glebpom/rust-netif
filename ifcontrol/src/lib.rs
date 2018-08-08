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

pub struct Iface {
    ifname: String,
}

impl Iface {
    pub fn find_by_name(ifname: &str) -> Result<Iface> {
        let ctl_fd = impls::new_control_socket()?;
        impls::get_iface_ifreq(&ctl_fd, ifname)?;
        Ok(Iface {
            ifname: ifname.to_owned(),
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
}
