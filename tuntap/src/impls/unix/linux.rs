use super::linux_common::*;
use ifcontrol::Iface;
use ifstructs::{ifreq, IfFlags};
use impls::unix::linux_common::TunTapFlags;
use impls::unix::*;
use libc::{c_short, c_uchar, IFF_MULTI_QUEUE, IFF_NO_PI, IFF_TAP, IFF_TUN, IFNAMSIZ};
use nix::fcntl;
use std::fs::File;
use std::fs::OpenOptions;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::reactor::PollEvented2;
use TunTapError;

pub struct Native {}

impl Native {
    pub fn new() -> Native {
        Native {}
    }

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

    pub fn create_tun_async(&self, name: Option<&str>, queues: usize) -> Result<::Virtualnterface<PollEvented2<super::EventedDescriptor<Native>>>, TunTapError> {
        let (files, name) = self.create(name, ::VirtualInterfaceType::Tun, true, queues)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: files.into_iter().map(|f| PollEvented2::new(super::EventedDescriptor(::Descriptor::from_file(f, &info)))).collect(),
            info: Arc::downgrade(&info),
        })
    }

    pub fn create_tap_async(&self, name: Option<&str>, queues: usize) -> Result<::Virtualnterface<PollEvented2<super::EventedDescriptor<Native>>>, TunTapError> {
        let (files, name) = self.create(name, ::VirtualInterfaceType::Tap, true, queues)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tap,
        }));
        Ok(::Virtualnterface {
            queues: files.into_iter().map(|f| PollEvented2::new(super::EventedDescriptor(::Descriptor::from_file(f, &info)))).collect(),
            info: Arc::downgrade(&info),
        })
    }
}

impl Native {
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

impl ::DescriptorCloser for Native {
    fn close_descriptor(_: &mut ::Descriptor<Native>) -> Result<(), TunTapError> {
        Ok(())
    }
}
