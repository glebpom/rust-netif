#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use self::apple::*;
#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use self::linux::*;

use std::os::unix::io::{AsRawFd, RawFd};

impl<C> AsRawFd for ::Descriptor<C>
where
    C: ::DescriptorCloser,
{
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}
