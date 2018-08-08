use impls::unix::*;
use libc;

#[allow(non_camel_case_types)]
pub type caddr_t = *mut libc::c_char;

// #define	SIOCGIFFLAGS	_IOWR('i', 17, struct ifreq)	/* get ifnet flags */
ioctl_readwrite!(iface_get_flags, b'i', 17, ifreq);

// #define	SIOCSIFFLAGS	 _IOW('i', 16, struct ifreq)	/* set ifnet flags */
ioctl_write_ptr!(iface_set_flags, b'i', 16, ifreq);
