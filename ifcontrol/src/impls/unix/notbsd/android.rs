use ifstructs::ifreq;

// #define SIOCGIFNAME 0x8910
ioctl_readwrite_bad!(ioctl_get_iface_name, 0x8910, ifreq);
