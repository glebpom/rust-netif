use super::bsd_common::*;
use errors::{ErrorKind, Result};
use impls::unix::*;
use libc;
use libc::EBUSY;
use nix;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::socket::{
    connect, getsockopt, socket, AddressFamily, GetSockOpt, SockAddr, SockFlag, SockProtocol,
    SockType,
};
use std::fs::File;
use std::fs::OpenOptions;
use std::os::unix::io::FromRawFd;
use std::path::Path;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::reactor::PollEvented2;
use ifcontrol::Iface;

// #define	SIOCAIFADDR	_IOW('i', 26, struct ifaliasreq)/* add/chg IF alias */
// ioctl_write_ptr!(iface_add_addr, b'i', 26, ifaliasreq);
// #define	SIOCDIFADDR	_IOW('i', 25, struct ifreq)	/* delete IF addr */
// ioctl_write_ptr!(iface_del_addr, b'i', 25, ifreq);

// #[derive(Copy, Clone)]
// #[repr(C)]
// pub struct ifaliasreq {
//     pub ifra_name: [u8; IFNAMSIZ],
//     pub ifra_addr: libc::sockaddr,
//     pub ifra_broadaddr: libc::sockaddr,
//     pub ifra_mask: libc::sockaddr,
// }

#[derive(Copy, Clone)]
struct UtunCreatedIfaceName;

impl GetSockOpt for UtunCreatedIfaceName {
    type Val = String;

    fn get(&self, fd: RawFd) -> nix::Result<String> {
        unsafe {
            let mut buf: [u8; libc::IFNAMSIZ] = [0; libc::IFNAMSIZ];
            let mut len = buf.len() as u32;

            let res = libc::getsockopt(
                fd,
                libc::SYSPROTO_CONTROL,
                libc::UTUN_OPT_IFNAME,
                buf.as_mut_ptr() as *mut libc::c_void,
                &mut len,
            );

            if res != 0 {
                return Err(nix::Error::last());
            };
            let name = String::from_utf8_lossy(&buf[..(len - 1) as usize]);
            Ok(name.to_string())
        }
    }
}

pub struct Native {}

impl Native {
    pub fn new() -> Native {
        Native {}
    }
}

impl Default for Native {
    fn default() -> Native {
        Native::new()
    }
}

// UTUN native OSX interface support
impl Native {
    pub fn create_tun(&self, unit: u32) -> Result<::Virtualnterface<::Descriptor<Native>>> {
        let (fd, name) = self.create_tun_inner(unit, false)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name: name.clone(),
            iface_type: ::VirtualInterfaceType::Tun,
        }));

        // add_addr_to_iface(
        //     &name,
        //     "192.168.33.1".parse().unwrap(),
        //     "255.255.255.0".parse().unwrap(),
        //     "192.168.33.255".parse().unwrap(),
        // )?;

        // del_addr_from_iface(
        //     &name,
        //     "192.168.33.1".parse().unwrap(),
        // )?;

        Ok(::Virtualnterface {
            queues: vec![::Descriptor::from_file(
                unsafe { File::from_raw_fd(fd) },
                &info,
            )],
            info: Arc::downgrade(&info),
        })
    }

    pub fn create_tun_async(
        &self,
        unit: u32,
    ) -> Result<::Virtualnterface<PollEvented2<super::EventedDescriptor<Native>>>> {
        let (fd, name) = self.create_tun_inner(unit, true)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: vec![PollEvented2::new(
                ::Descriptor::from_file(unsafe { File::from_raw_fd(fd) }, &info).into(),
            )],
            info: Arc::downgrade(&info),
        })
    }

    fn create_tun_inner(&self, unit: u32, is_async: bool) -> Result<(RawFd, String)> {
        let fd: RawFd = socket(
            AddressFamily::System,
            SockType::Datagram,
            SockFlag::empty(),
            SockProtocol::KextControl,
        )?;

        let addr = SockAddr::new_sys_control(fd, "com.apple.net.utun_control", unit)?;
        //TODO: close fd if error ?

        if is_async {
            fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK))?;
        }

        connect(fd, &addr)?;

        let name = getsockopt(fd, UtunCreatedIfaceName)?;

        Ok((fd, name))
    }
}

impl ::DescriptorCloser for Native {
    fn close_descriptor(_: &mut ::Descriptor<Native>) -> Result<()> {
        Ok(())
    }
}

pub struct DevPath {
    pub dev_name: String,
    pub max_num: usize,
}

pub struct TunTapOsx {
    tun: Option<DevPath>,
    tap: Option<DevPath>,
}

impl TunTapOsx {
    pub fn new(tun: Option<DevPath>, tap: Option<DevPath>) -> TunTapOsx {
        TunTapOsx { tun, tap }
    }
}

impl Default for TunTapOsx {
    fn default() -> TunTapOsx {
        TunTapOsx {
            tun: Some(DevPath {
                dev_name: "tun".to_owned(),
                max_num: 16,
            }),
            tap: Some(DevPath {
                dev_name: "tap".to_owned(),
                max_num: 16,
            }),
        }
    }
}

macro_rules! probe {
    ($iface_type:expr) => {
        match $iface_type {
            Some(DevPath {
                ref dev_name,
                max_num: _,
            }) => {
                let path_str = format!("/dev/{}0", dev_name);
                let path = Path::new(&path_str);
                return path.exists();
            }
            None => return false,
        }
    };
}

macro_rules! create_descriptor {
    ($iface_type:expr, $virtual_iface_type:expr, $iface_name:expr, $dev_idx:expr) => {
        match $iface_type {
            Some(DevPath {
                ref dev_name,
                max_num,
            }) => {
                let (f, name) = if let Some(idx) = $dev_idx {
                    if let Some(r) = TunTapOsx::try_create_by_idx(&dev_name, idx) {
                        r?
                    } else {
                        return Err(ErrorKind::Busy.into());
                    }
                } else {
                    TunTapOsx::create_tun_tap_driver(&dev_name, max_num)?
                };
                let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
                    name,
                    iface_type: $virtual_iface_type,
                }));

                return Ok(::Virtualnterface {
                    queues: vec![::Descriptor::from_file(f, &info)],
                    info: Arc::downgrade(&info),
                });
            }
            None => bail!(ErrorKind::NotSupported(format!(
                "{} is not configured in backend",
                stringify!($iface_name)
            ))),
        }
    };
}

impl TunTapOsx {
    pub fn probe_tun(&self) -> bool {
        probe!(self.tun)
    }

    pub fn probe_tap(&self) -> bool {
        probe!(self.tap)
    }

    pub fn create_tun(
        &self,
        dev_idx: Option<usize>,
    ) -> Result<::Virtualnterface<::Descriptor<TunTapOsx>>> {
        create_descriptor!(self.tun, ::VirtualInterfaceType::Tun, tun, dev_idx)
    }

    pub fn create_tap(
        &self,
        dev_idx: Option<usize>,
    ) -> Result<::Virtualnterface<::Descriptor<TunTapOsx>>> {
        create_descriptor!(self.tap, ::VirtualInterfaceType::Tap, tap, dev_idx)
    }

    fn try_create_by_idx(dev_name: &str, dev_idx: usize) -> Option<Result<(File, String)>> {
        let path_str = format!("/dev/{}{}", dev_name, dev_idx);
        let path = Path::new(&path_str);
        if !path.exists() {
            return Some(Err(ErrorKind::NotSupported(format!(
                "'{}' not exists",
                path.to_string_lossy()
            )).into()));
        }

        let f = OpenOptions::new().read(true).write(true).open(path);
        if let Err(e) = f {
            if let Some(raw_error) = e.raw_os_error() {
                if raw_error == EBUSY {
                    return None;
                }
            }
            return Some(Err(e.into()));
        }

        let name = format!("{}{}", dev_name, dev_idx);

        match Iface::find_by_name(&name) {
            Err(e) =>  {
                return Some(Err(e.into()));
            }
            Ok(iface) => {
                if let Err(e) = iface.up() {
                    return Some(Err(e.into()));
                }
            }
        }

        match f {
            Err(e) => Some(Err(e.into())),
            Ok(f) => Some(Ok((f, name))),
        }
    }

    fn create_tun_tap_driver(dev_name: &str, max_num: usize) -> Result<(File, String)> {
        for i in 0..max_num {
            if let Some(res) = Self::try_create_by_idx(dev_name, i) {
                return res;
            }
        }
        return Err(ErrorKind::MaxNumberReached(max_num).into());
    }
}

impl ::DescriptorCloser for TunTapOsx {
    fn close_descriptor(_: &mut ::Descriptor<TunTapOsx>) -> Result<()> {
        Ok(())
    }
}
