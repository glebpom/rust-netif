use ifstructs::ifaliasreq;

// #define	SIOCAIFADDR	 _IOW('i', 43, struct ifaliasreq)/* add/chg IF alias */
ioctl_write_ptr!(iface_add_addr, b'i', 43, ifaliasreq);
