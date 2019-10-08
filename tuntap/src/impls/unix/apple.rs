use crate::evented::EventedDescriptor;
use crate::poll_evented::PollEvented;
use ifcontrol::Iface;
use impls::unix::*;
use libc;
use libc::EBUSY;
use nix;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::socket::{
    connect, getsockopt, socket, AddressFamily, GetSockOpt, SockAddr, SockFlag, SockProtocol,
    SockType,
};
use parking_lot::Mutex;
use std::fs::File;
use std::fs::OpenOptions;
use std::os::unix::io::FromRawFd;
use std::os::unix::prelude::*;
use std::path::Path;
use std::str;
use std::sync::Arc;
use TunTapError;

#[derive(Copy, Clone)]
#[cfg(target_os = "macos")]
struct UtunCreatedIfaceName;

#[cfg(target_os = "macos")]
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

    pub unsafe fn tun_async_from_fds(
        &self,
        ifname: &str,
        fds: &[RawFd],
    ) -> Result<::Virtualnterface<PollEvented<EventedDescriptor<Native>>>, TunTapError> {
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name: ifname.to_owned(),
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: fds
                .iter()
                .map(|&fd| {
                    PollEvented::new(::Descriptor::from_file(File::from_raw_fd(fd), &info).into())
                })
                .collect(),
            info: Arc::downgrade(&info),
        })
    }

    pub unsafe fn tun_from_fds(
        &self,
        ifname: &str,
        fds: &[RawFd],
    ) -> Result<::Virtualnterface<::Descriptor<Native>>, TunTapError> {
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name: ifname.to_owned(),
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: fds
                .iter()
                .map(|&fd| ::Descriptor::from_file(File::from_raw_fd(fd), &info))
                .collect(),
            info: Arc::downgrade(&info),
        })
    }
}

impl Default for Native {
    fn default() -> Native {
        Native::new()
    }
}

// UTUN native OSX interface support
#[cfg(target_os = "macos")]
impl Native {
    pub fn create_tun(
        &self,
        unit: u32,
    ) -> Result<::Virtualnterface<::Descriptor<Native>>, TunTapError> {
        let (fd, name) = self.create_tun_inner(unit, false)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name: name.clone(),
            iface_type: ::VirtualInterfaceType::Tun,
        }));

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
    ) -> Result<::Virtualnterface<PollEvented<EventedDescriptor<Native>>>, TunTapError> {
        let (fd, name) = self.create_tun_inner(unit, true)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: vec![PollEvented::new(
                ::Descriptor::from_file(unsafe { File::from_raw_fd(fd) }, &info).into(),
            )],
            info: Arc::downgrade(&info),
        })
    }

    fn create_tun_inner(&self, unit: u32, is_async: bool) -> Result<(RawFd, String), TunTapError> {
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
    fn close_descriptor(_: &mut ::Descriptor<Native>) -> Result<(), TunTapError> {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub struct DevPath {
    pub dev_name: String,
    pub max_num: usize,
}

#[cfg(target_os = "macos")]
pub struct TunTapOsx {
    tun: Option<DevPath>,
    tap: Option<DevPath>,
}

#[cfg(target_os = "macos")]
impl TunTapOsx {
    pub fn new(tun: Option<DevPath>, tap: Option<DevPath>) -> TunTapOsx {
        TunTapOsx { tun, tap }
    }
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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
                        return Err(TunTapError::Busy);
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
            None => {
                return Err(TunTapError::NotSupported {
                    msg: format!("{} is not configured in backend", stringify!($iface_name)),
                });
            }
        }
    };
}

#[cfg(target_os = "macos")]
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
    ) -> Result<::Virtualnterface<::Descriptor<TunTapOsx>>, TunTapError> {
        create_descriptor!(self.tun, ::VirtualInterfaceType::Tun, tun, dev_idx)
    }

    pub fn create_tap(
        &self,
        dev_idx: Option<usize>,
    ) -> Result<::Virtualnterface<::Descriptor<TunTapOsx>>, TunTapError> {
        create_descriptor!(self.tap, ::VirtualInterfaceType::Tap, tap, dev_idx)
    }

    fn try_create_by_idx(
        dev_name: &str,
        dev_idx: usize,
    ) -> Option<Result<(File, String), TunTapError>> {
        let path_str = format!("/dev/{}{}", dev_name, dev_idx);
        let path = Path::new(&path_str);
        if !path.exists() {
            return Some(Err(TunTapError::NotSupported {
                msg: format!("'{}' not exists", path.to_string_lossy()),
            }));
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
            Err(e) => {
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

    fn create_tun_tap_driver(
        dev_name: &str,
        max_num: usize,
    ) -> Result<(File, String), TunTapError> {
        for i in 0..max_num {
            if let Some(res) = Self::try_create_by_idx(dev_name, i) {
                return res;
            }
        }
        return Err(TunTapError::MaxNumberReached { max: max_num });
    }
}

#[cfg(target_os = "macos")]
impl ::DescriptorCloser for TunTapOsx {
    fn close_descriptor(_: &mut ::Descriptor<TunTapOsx>) -> Result<(), TunTapError> {
        Ok(())
    }
}
