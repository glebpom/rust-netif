use libc;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ifmap {
    pub mem_start: libc::c_ulong,
    pub mem_end: libc::c_ulong,
    pub base_addr: libc::c_ushort,
    pub irq: libc::c_uchar,
    pub dma: libc::c_uchar,
    pub port: libc::c_uchar,
}

cfg_if! {
    if #[cfg(target_os = "android")] {
        mod android;
        pub use self::android::*;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        pub use self::linux::*;
    } else {
        // Unknown target_os
    }
}