use errors::{ErrorKind, Result};
use ifstructs::{ifreq, rtentry};
use libc;
use std::ffi::CString;
use std::os::unix::io::AsRawFd;
use std::mem;

// #define SIOCGIFNAME	0x8910		/* get iface name		*/
ioctl_readwrite_bad!(ioctl_get_iface_name, 0x890B, ifreq);

// #define SIOCADDRT	0x890B		/* add routing table entry	*/
ioctl_write_ptr_bad!(ioctl_add_routing_entry, 0x890B, rtentry);
// #define SIOCDELRT	0x890C		/* delete routing table entry	*/
ioctl_write_ptr_bad!(ioctl_del_routing_entry, 0x890C, rtentry);

// #define SIOCBRADDBR     0x89a0		/* create new bridge device     */
ioctl_write_ptr_bad!(ioctl_create_bridge, 0x89a0, libc::c_char);
// #define SIOCBRDELBR     0x89a1		/* remove bridge device         */
ioctl_write_ptr_bad!(ioctl_remove_bridge, 0x89a1, libc::c_char);

// #define SIOCBRADDIF	0x89a2		/* add interface to bridge      */
ioctl_write_ptr_bad!(ioctl_add_iface_to_bridge, 0x89a2, ifreq);
// #define SIOCBRDELIF	0x89a3		/* remove interface from bridge */
ioctl_write_ptr_bad!(ioctl_remove_iface_from_bridge, 0x89a3, ifreq);


pub fn create_bridge<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<()> {
    let s = CString::new(ifname).unwrap();
    unsafe { ioctl_create_bridge(ctl_fd.as_raw_fd(), s.as_ptr())? };
    Ok(())
}

pub fn remove_bridge<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<()> {
    let s = CString::new(ifname).unwrap();
    unsafe { ioctl_remove_bridge(ctl_fd.as_raw_fd(), s.as_ptr())? };
    Ok(())
}

pub fn add_iface_to_bridge<F: AsRawFd>(
    ctl_fd: &F,
    bridge_ifname: &str,
    iface_ifname: &str,
) -> Result<()> {
    let iface_idx = get_iface_index(ctl_fd, iface_ifname)?;
    let mut req = ifreq::from_name(bridge_ifname)?;
    req.ifr_ifru.ifr_ifindex = iface_idx;
    unsafe { ioctl_add_iface_to_bridge(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(())
}

pub fn remove_iface_from_bridge<F: AsRawFd>(
    ctl_fd: &F,
    bridge_ifname: &str,
    iface_ifname: &str,
) -> Result<()> {
    let iface_idx = get_iface_index(ctl_fd, iface_ifname)?;
    let mut req = ifreq::from_name(bridge_ifname)?;
    req.ifr_ifru.ifr_ifindex = iface_idx;
    unsafe { ioctl_remove_iface_from_bridge(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(())
}

pub fn bind_to_device<S: AsRawFd>(socket: &S, iface_name: &str) -> Result<()> {
    let cstr = CString::new(iface_name).unwrap();
    let res = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_BINDTODEVICE,
            cstr.as_bytes().as_ptr() as *const _ as *const _,
            iface_name.len() as u32,
        )
    };
    if res != 0 {
        bail!(::std::io::Error::last_os_error());
    }
    Ok(())
}

pub fn get_iface_name<F: AsRawFd>(ctl_fd: &F, idx: libc::c_int) -> Result<String> {
    let mut req: ifreq = unsafe { mem::zeroed() };
    unsafe { req.set_iface_index(idx) };
    unsafe { ::impls::ioctl_get_iface_index(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(req.get_name()?)
}

pub fn get_iface_index<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<libc::c_int> {
    let mut req = ifreq::from_name(ifname)?;
    unsafe { ::impls::ioctl_get_iface_index(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(unsafe { req.ifr_ifru.ifr_ifindex })
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_bridge() {
//         let ctl_fd = ::impls::new_control_socket().unwrap();
//         create_bridge(&ctl_fd, "lcstrbr0").expect("create bridge");
//         ::Iface::find_by_name("lcstrbr0").unwrap().up().expect("up bridge");
//         println!("iface index = {}", get_iface_index(&ctl_fd, "tap1").expect("idx"));
//         add_iface_to_bridge(&ctl_fd, "lcstrbr0", "tap1").expect("add tap1 to bridge");
//         remove_iface_from_bridge(&ctl_fd, "lcstrbr0", "tap1").expect("remove tap1 from bridge");
//         ::Iface::find_by_name("lcstrbr0").unwrap().down().expect("down bridge");
//         remove_bridge(&ctl_fd, "lcstrbr0").expect("remove bridge");
//     }
// }
