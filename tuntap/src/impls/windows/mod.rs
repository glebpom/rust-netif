use bytes::BufMut;
use ipconfig;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::Cursor;
use std::net::Ipv4Addr;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr;
use std::sync::{Arc, Mutex};
use winapi::ctypes::c_void;
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winioctl::FILE_DEVICE_UNKNOWN;
use winapi::um::winnt::{
    FILE_ATTRIBUTE_SYSTEM, FILE_SHARE_READ, FILE_SHARE_WRITE, MAXIMUM_REPARSE_DATA_BUFFER_SIZE,
};
use winreg::enums::*;
use winreg::RegKey;
use TunTapError;

macro_rules! CTL_CODE {
    ($DeviceType:expr, $Function:expr, $Method:expr, $Access:expr) => {
        ($DeviceType << 16) | ($Access << 14) | ($Function << 2) | $Method
    };
}

pub struct Native {}

impl ::DescriptorCloser for Native {
    fn close_descriptor(_: &mut ::Descriptor<Native>) -> Result<(), TunTapError> {
        Ok(())
    }
}

impl Native {
    pub fn new() -> Native {
        Native {}
    }

    pub fn open_tun(
        &self,
        device_id: &str,
        ip: Ipv4Addr,
        netmask: Ipv4Addr,
        gateway: Ipv4Addr,
    ) -> Result<::Virtualnterface<::Descriptor<Native>>, TunTapError> {
        let (file, name) = Native::open_dev(device_id, Some((ip, netmask, gateway)))?;

        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tun,
        }));

        Ok(::Virtualnterface {
            queues: vec![::Descriptor::from_file(file, &info)],
            info: Arc::downgrade(&info),
        })
    }

    pub fn open_tap(
        &self,
        device_id: &str,
    ) -> Result<::Virtualnterface<::Descriptor<Native>>, TunTapError> {
        let (file, name) = Native::open_dev(device_id, None)?;

        let info = Arc::new(Mutex::new(::VirtualInterfaceInfo {
            name,
            iface_type: ::VirtualInterfaceType::Tap,
        }));

        Ok(::Virtualnterface {
            queues: vec![::Descriptor::from_file(file, &info)],
            info: Arc::downgrade(&info),
        })
    }
}

impl Native {
    fn get_device_id(req_component_id: &str) -> Result<Option<String>, TunTapError> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let base_path = Path::new("SYSTEM")
            .join("CurrentControlSet")
            .join("Control")
            .join("Class")
            .join("{4D36E972-E325-11CE-BFC1-08002BE10318}");
        let subkey = hklm.open_subkey(&base_path)?;
        for res in subkey.enum_keys() {
            let k = res?;
            let sk = hklm.open_subkey(&base_path.join(k))?;
            let component_id: String = sk.get_value("ComponentId")?;
            if req_component_id == component_id {
                let net_cfg_instance_id: String = sk.get_value("NetCfgInstanceId")?;
                return Ok(Some(net_cfg_instance_id));
            }
        }
        Ok(None)
    }

    fn open_dev(
        device_id: &str,
        tun_args: Option<(Ipv4Addr, Ipv4Addr, Ipv4Addr)>,
    ) -> Result<(File, String), TunTapError> {
        match Native::get_device_id(device_id)? {
            None => {
                return Err(TunTapError::DriverNotFound {
                    msg: format!("device_id {} not found", device_id),
                });
            }
            Some(device_id) => {
                let path = format!("\\\\.\\Global\\\\{}.tap", device_id);
                let mut file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .append(false)
                    .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                    .attributes(FILE_ATTRIBUTE_SYSTEM)
                    .create(false)
                    .truncate(false)
                    .create_new(false)
                    .open(path)?;

                let mut mac = [0u8; 6];
                let mut bytes_returned = 0u32;

                if unsafe {
                    DeviceIoControl(
                        file.as_raw_handle() as *mut _ as *mut c_void,
                        CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x01, 0, 0),
                        &mut mac as *mut _ as *mut c_void,
                        6,
                        &mut mac as *mut _ as *mut c_void,
                        6,
                        &mut bytes_returned,
                        ptr::null_mut(),
                    )
                } == 0
                {
                    return Err(io::Error::last_os_error().into());
                }

                let mac = mac[..(bytes_returned as usize)].to_vec();

                //Up Device
                let mut rbuf = [0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize];
                let mut code = [1u8, 0u8, 0u8, 0u8]; //UP
                if unsafe {
                    DeviceIoControl(
                        file.as_raw_handle() as *mut _ as *mut c_void,
                        CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x06, 0, 0),
                        &mut code as *mut _ as *mut c_void,
                        4,
                        &mut rbuf as *mut _ as *mut c_void,
                        MAXIMUM_REPARSE_DATA_BUFFER_SIZE,
                        &mut bytes_returned,
                        ptr::null_mut(),
                    )
                } == 0
                {
                    return Err(io::Error::last_os_error().into());
                }

                if let Some((ip, netmask, gateway)) = tun_args {
                    //Set TUN
                    let mut rbuf = [0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize];
                    let mut code = [0u8; 12]; // = [192, 168, 66, 12, 192, 168, 66, 0, 255, 255, 255, 0]; //UP
                    let mut c = Cursor::new(code);
                    c.put_u32_be(ip.into());
                    c.put_u32_be(netmask.into());
                    c.put_u32_be(gateway.into());

                    if unsafe {
                        DeviceIoControl(
                            file.as_raw_handle() as *mut _ as *mut c_void,
                            CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x0A, 0, 0),
                            &mut code as *mut _ as *mut c_void,
                            12,
                            &mut rbuf as *mut _ as *mut c_void,
                            MAXIMUM_REPARSE_DATA_BUFFER_SIZE,
                            &mut bytes_returned,
                            ptr::null_mut(),
                        )
                    } == 0
                    {
                        return Err(io::Error::last_os_error().into());
                    }
                }

                let adapters = ipconfig::get_adapters().map_err(|_| TunTapError::Other {
                    msg: "failed to get adapters list".to_owned(),
                })?;
                let maybe_adapter_name = adapters
                    .iter()
                    .filter(|adapter| {
                        if let &Some(ref physical_address) = adapter.physical_address() {
                            if *physical_address == mac {
                                return true;
                            }
                        }
                        false
                    }).next()
                    .map(|a| a.friendly_name());

                if let Some(adapter_name) = maybe_adapter_name {
                    return Ok((file, adapter_name.clone()));
                } else {
                    return Err(TunTapError::Other {
                        msg: "failed to find created interface".to_owned(),
                    });
                }
            }
        }
    }
}
