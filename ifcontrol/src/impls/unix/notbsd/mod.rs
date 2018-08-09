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

use libc;
use ifstructs::ifreq;

// #define SIOCGIFFLAGS	0x8913		/* get flags			*/
ioctl_readwrite_bad!(iface_get_flags, libc::SIOCGIFFLAGS, ifreq);
// #define SIOCSIFFLAGS	0x8914		/* set flags			*/
ioctl_write_ptr_bad!(iface_set_flags, libc::SIOCSIFFLAGS, ifreq);
// #define SIOCGIFHWADDR	0x8927
ioctl_write_ptr_bad!(iface_get_hwaddr, libc::SIOCGIFHWADDR, ifreq);
