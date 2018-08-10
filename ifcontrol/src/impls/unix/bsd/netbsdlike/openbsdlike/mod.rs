cfg_if! {
    if #[cfg(target_os = "openbsd")] {
        mod openbsd;
        pub use self::openbsd::*;
    } else {
        // Unknown target_os
    }
}
