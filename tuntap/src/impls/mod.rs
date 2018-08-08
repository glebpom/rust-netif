#[cfg(any(target_os = "freebsd", target_os = "linux", target_os = "android", target_os = "macos"))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(
    any(target_os = "freebsd", target_os = "linux", target_os = "android", target_os = "macos")
)]
pub use self::unix::*;
#[cfg(windows)]
pub use self::windows::*;
