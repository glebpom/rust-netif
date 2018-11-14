#[cfg(any(target_os = "freebsd", target_os = "linux", target_os = "macos", target_os = "android"))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(any(target_os = "freebsd", target_os = "linux", target_os = "macos", target_os = "android"))]
pub use self::unix::*;
#[cfg(windows)]
pub use self::windows::*;
