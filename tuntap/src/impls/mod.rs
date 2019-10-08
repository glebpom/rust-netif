#[cfg(any(
    target_os = "freebsd",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(any(
    target_os = "freebsd",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
))]
pub use self::unix::*;
#[cfg(windows)]
pub use self::windows::*;
