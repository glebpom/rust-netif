use ifstructs::{ifreq};
use libc;
use errors::{Result, ErrorKind};
use std::ffi::CString;
use std::os::unix::io::AsRawFd;

// #define SIOCGIFINDEX	0x8933		/* name -> if_index mapping	*/
ioctl_readwrite_bad!(ioctl_get_iface_index, 0x8933, ifreq);

// #define SIOCBRADDBR     0x89a0		/* create new bridge device     */
ioctl_write_ptr_bad!(ioctl_create_bridge, 0x89a0, libc::c_char);
// #define SIOCBRDELBR     0x89a1		/* remove bridge device         */
ioctl_write_ptr_bad!(ioctl_remove_bridge, 0x89a1, libc::c_char);


// #define SIOCBRADDIF	0x89a2		/* add interface to bridge      */
ioctl_write_ptr_bad!(ioctl_add_iface_to_bridge, 0x89a2, ifreq);
// #define SIOCBRDELIF	0x89a3		/* remove interface from bridge */
ioctl_write_ptr_bad!(ioctl_remove_iface_from_bridge, 0x89a3, ifreq);

pub fn get_iface_index<F: AsRawFd>(ctl_fd: &F, ifname: &str) -> Result<libc::c_int> {
    let mut req = ifreq::from_name(ifname)?;
    unsafe { ioctl_get_iface_index(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(unsafe { req.ifr_ifru.ifr_ifindex })
}

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

pub fn add_iface_to_bridge<F: AsRawFd>(ctl_fd: &F, bridge_ifname: &str, iface_ifname: &str) -> Result<()> {
    let iface_idx = get_iface_index(ctl_fd, iface_ifname)?;
    let mut req = ifreq::from_name(bridge_ifname)?;
    req.ifr_ifru.ifr_ifindex = iface_idx;
    unsafe { ioctl_add_iface_to_bridge(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(())
}

pub fn remove_iface_from_bridge<F: AsRawFd>(ctl_fd: &F, bridge_ifname: &str, iface_ifname: &str) -> Result<()> {
    let iface_idx = get_iface_index(ctl_fd, iface_ifname)?;
    let mut req = ifreq::from_name(bridge_ifname)?;
    req.ifr_ifru.ifr_ifindex = iface_idx;
    unsafe { ioctl_remove_iface_from_bridge(ctl_fd.as_raw_fd(), &mut req)? };
    Ok(())
}


// bitflags! {
//     // https://github.com/torvalds/linux/blob/master/include/uapi/linux/route.h
//     // u16
//     pub struct Flags: i32 {
//         const RTF_UP        = sys::RTF_UP as i32;
//         const RTF_GATEWAY   = sys::RTF_GATEWAY as i32;
//         const RTF_HOST      = sys::RTF_HOST as i32;
//         const RTF_REINSTATE = sys::RTF_REINSTATE as i32;
//         const RTF_DYNAMIC   = sys::RTF_DYNAMIC as i32;
//         const RTF_MODIFIED  = sys::RTF_MODIFIED as i32;
//         const RTF_MTU       = sys::RTF_MTU as i32;
//         const RTF_MSS       = sys::RTF_MSS as i32;
//         const RTF_WINDOW    = sys::RTF_WINDOW as i32;
//         const RTF_IRTT      = sys::RTF_IRTT as i32;
//         const RTF_REJECT    = sys::RTF_REJECT as i32;
//     }
// }

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
