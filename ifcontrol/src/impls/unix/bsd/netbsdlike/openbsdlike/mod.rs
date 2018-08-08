cfg_if! {
    if #[cfg(target_os = "openbsd")] {
        mod openbsd;
        pub use self::openbsd::*;
    } else if #[cfg(target_os = "bitrig")] {
        mod bitrig;
        pub use self::bitrig::*;
    } else {
        // Unknown target_os
    }
}
