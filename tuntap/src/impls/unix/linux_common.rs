use ifstructs::ifreq;
use impls::unix::*;
use libc;

ioctl_write_ptr!(tun_set_iff, b'T', 202, libc::c_int);
// #define SIOCGIFFLAGS	0x8913		/* get flags			*/
ioctl_readwrite_bad!(iface_get_flags, libc::SIOCGIFFLAGS, ifreq);
// #define SIOCSIFFLAGS	0x8914		/* set flags			*/
ioctl_write_ptr_bad!(iface_set_flags, libc::SIOCSIFFLAGS, ifreq);
