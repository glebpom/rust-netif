use crate::evented::EventedDescriptor;
use std::fs::File;
use std::os::unix::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::reactor::PollEvented2;
use TunTapError;

pub struct Native {}

impl Native {
    pub fn new() -> Native {
        Native {}
    }

    pub unsafe fn tun_async_from_fds(&self, ifname: &str, fds: &[RawFd]) -> Result<::Virtualnterface<PollEvented2<EventedDescriptor<Native>>>, TunTapError> {
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name: ifname.to_owned(),
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: fds
                .iter()
                .map(|&fd| PollEvented2::new(::Descriptor::from_file(File::from_raw_fd(fd), &info).into()))
                .collect(),
            info: Arc::downgrade(&info),
        })
    }

    pub unsafe fn tun_from_fds(&self, ifname: &str, fds: &[RawFd]) -> Result<::Virtualnterface<::Descriptor<Native>>, TunTapError> {
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name: ifname.to_owned(),
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: fds.iter().map(|&fd| ::Descriptor::from_file(File::from_raw_fd(fd), &info)).collect(),
            info: Arc::downgrade(&info),
        })
    }
}

impl ::DescriptorCloser for Native {
    fn close_descriptor(_: &mut ::Descriptor<Native>) -> Result<(), TunTapError> {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod normal_linux {
    use super::*;
    use ifcontrol::Iface;
    use ifstructs::{ifreq, IfFlags};
    use libc;
    use nix::fcntl;
    use std::fs::OpenOptions;
    use std::mem;
    use std::os::unix::io::AsRawFd;
    use std::path::Path;
    use std::str;

    bitflags! {
        pub struct TunTapFlags: libc::c_short {
            const IFF_TUN = 0x0001;
            const IFF_TAP = 0x0002;
            const IFF_NAPI = 0x0010;
            const IFF_NAPI_FRAGS = 0x0020;
            const IFF_NO_PI = 0x1000;
            /// this flag has no real effect
            const IFF_ONE_QUEUE = 0x2000;
            const IFF_VNET_HDR = 0x4000;
            #[allow(overflowing_literals)]
            const IFF_TUN_EXCL = 0x8000;
            const IFF_MULTI_QUEUE = 0x0100;
            const IFF_ATTACH_QUEUE = 0x0200;
            const IFF_DETACH_QUEUE = 0x0400;
            /// read-only flag
            const IFF_PERSIST = 0x0800;
            const IFF_NOFILTER = 0x1000;
        }
    }

    ioctl_write_ptr!(tun_set_iff, b'T', 202, libc::c_int);

    impl Native {
        pub fn create_tun(&self, name: Option<&str>, queues: usize) -> Result<::Virtualnterface<::Descriptor<Native>>, TunTapError> {
            let (files, name) = self.create(name, ::VirtualInterfaceType::Tun, false, queues)?;
            let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
                name,
                iface_type: ::VirtualInterfaceType::Tun,
            }));

            Ok(::Virtualnterface {
                queues: files.into_iter().map(|f| ::Descriptor::from_file(f, &info)).collect(),
                info: Arc::downgrade(&info),
            })
        }

        pub fn create_tap(&self, name: Option<&str>, queues: usize) -> Result<::Virtualnterface<::Descriptor<Native>>, TunTapError> {
            let (files, name) = self.create(name, ::VirtualInterfaceType::Tap, false, queues)?;
            let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
                name,
                iface_type: ::VirtualInterfaceType::Tap,
            }));
            Ok(::Virtualnterface {
                queues: files.into_iter().map(|f| ::Descriptor::from_file(f, &info)).collect(),
                info: Arc::downgrade(&info),
            })
        }

        pub fn create_tun_async(&self, name: Option<&str>, queues: usize) -> Result<::Virtualnterface<PollEvented2<EventedDescriptor<Native>>>, TunTapError> {
            let (files, name) = self.create(name, ::VirtualInterfaceType::Tun, true, queues)?;
            let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
                name,
                iface_type: ::VirtualInterfaceType::Tun,
            }));
            Ok(::Virtualnterface {
                queues: files.into_iter().map(|f| PollEvented2::new(::Descriptor::from_file(f, &info).into())).collect(),
                info: Arc::downgrade(&info),
            })
        }

        pub fn create_tap_async(&self, name: Option<&str>, queues: usize) -> Result<::Virtualnterface<PollEvented2<EventedDescriptor<Native>>>, TunTapError> {
            let (files, name) = self.create(name, ::VirtualInterfaceType::Tap, true, queues)?;
            let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
                name,
                iface_type: ::VirtualInterfaceType::Tap,
            }));
            Ok(::Virtualnterface {
                queues: files.into_iter().map(|f| PollEvented2::new(::Descriptor::from_file(f, &info).into())).collect(),
                info: Arc::downgrade(&info),
            })
        }

        fn create_queue(&self, name: &str, flags: TunTapFlags, is_async: bool) -> Result<(File, String), TunTapError> {
            let path = Path::new("/dev/net/tun");

            let file = OpenOptions::new().read(true).write(true).open(&path)?;

            let mut req = ifreq::from_name(name)?;

            unsafe { req.set_raw_flags(flags.bits()) };

            unsafe { tun_set_iff(file.as_raw_fd(), &mut req as *mut _ as *mut _) }?;

            if is_async {
                fcntl::fcntl(file.as_raw_fd(), fcntl::FcntlArg::F_SETFL(fcntl::OFlag::O_NONBLOCK))?;
            }

            Ok((file, req.get_name()?))
        }

        fn create(&self, name: Option<&str>, iface_type: ::VirtualInterfaceType, is_async: bool, queues: usize) -> Result<(Vec<File>, String), TunTapError> {
            if let Some(ref s) = name {
                if s.is_empty() {
                    return Err(TunTapError::BadArguments { msg: "name is empty".to_owned() });
                }
            }

            if queues == 0 {
                return Err(TunTapError::BadArguments {
                    msg: "should be at least 1 queue".to_owned(),
                });
            }

            let mut flags = TunTapFlags::IFF_NO_PI;
            flags.insert(match iface_type {
                ::VirtualInterfaceType::Tun => TunTapFlags::IFF_TUN,
                ::VirtualInterfaceType::Tap => TunTapFlags::IFF_TAP,
            });
            if queues > 1 {
                flags.insert(TunTapFlags::IFF_MULTI_QUEUE);
            };

            let mut files = vec![];

            let (file, resulting_name) = self.create_queue(name.unwrap_or(""), flags, is_async)?;

            files.push(file);

            if queues > 1 {
                for _ in 0..queues - 1 {
                    files.push(self.create_queue(&resulting_name, flags, is_async)?.0);
                }
            }

            Iface::up_by_name(&resulting_name)?;

            Ok((files, resulting_name))
        }
    }
}

#[cfg(target_os = "linux")]
pub use self::normal_linux::*;
