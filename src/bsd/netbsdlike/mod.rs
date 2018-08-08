cfg_if! {
    if #[cfg(target_os = "netbsd")] {
        mod netbsd;
        pub use self::netbsd::*;
    } else if #[cfg(any(target_os = "openbsd", target_os = "bitrig"))] {
        mod openbsdlike;
        pub use self::openbsdlike::*;
    } else {
        // Unknown target_os
    }
}