use super::linux_common::*;
use errors::{ErrorKind, Result};
use ifcontrol::Iface;
use ifstructs::{ifreq, IfFlags};
use impls::unix::*;
use libc::{self, IFF_NO_PI, IFF_TUN, IFNAMSIZ};
use nix::fcntl;
use std::fs::File;
use std::fs::OpenOptions;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::reactor::PollEvented2;

pub struct Native {}

impl Native {
    pub fn new() -> Native {
        Native {}
    }

    pub fn create_tun(
        &self,
        name: Option<&str>,
    ) -> Result<::Virtualnterface<::Descriptor<Native>>> {
        let (file, name) = self.create(name, false)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tun,
        }));

        Ok(::Virtualnterface {
            queues: vec![::Descriptor::from_file(file, &info)],
            info: Arc::downgrade(&info),
        })
    }

    pub fn create_tun_async(
        &self,
        name: Option<&str>,
    ) -> Result<::Virtualnterface<PollEvented2<super::EventedDescriptor<Native>>>> {
        let (file, name) = self.create(name, true)?;
        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tun,
        }));
        Ok(::Virtualnterface {
            queues: vec![PollEvented2::new(super::EventedDescriptor(
                ::Descriptor::from_file(file, &info),
            ))],
            info: Arc::downgrade(&info),
        })
    }
}

impl Native {
    fn create(&self, name: Option<&str>, is_async: bool) -> Result<(File, String)> {
        if let Some(ref s) = name {
            if s.is_empty() {
                bail!(ErrorKind::BadArguments("name is empty".to_owned()));
            }
        }

        let path = Path::new("/dev/net/tun");

        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let mut req = ifreq::from_name(name.unwrap_or(""))?;

        let mut flags = unsafe { req.get_flags() };

        flags.insert(IfFlags::IFF_NO_PI);
        flags.insert(IfFlags::IFF_TUN);

        unsafe { req.set_flags(flags) };

        unsafe { tun_set_iff(file.as_raw_fd(), &mut req as *mut _ as *mut _) }?;

        if is_async {
            fcntl::fcntl(
                file.as_raw_fd(),
                fcntl::FcntlArg::F_SETFL(fcntl::OFlag::O_NONBLOCK),
            )?;
        }

        let resulting_name = req.get_name()?;

        Iface::find_by_name(&resulting_name)?.up()?;

        Ok((file, resulting_name))
    }
}

impl ::DescriptorCloser for Native {
    fn close_descriptor(_: &mut ::Descriptor<Native>) -> Result<()> {
        Ok(())
    }
}
