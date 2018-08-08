extern crate libc;
#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(any(target_os = "linux",
                 target_os = "android",
                 target_os = "fuchsia"))] {
        mod notbsd;
        pub use self::notbsd::*;
    } else if #[cfg(any(target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "dragonfly",
                        target_os = "openbsd",
                        target_os = "netbsd",
                        target_os = "bitrig"))] {
        mod bsd;
        pub use self::bsd::*;
    } else {
        // Unknown target_os
    }
}
