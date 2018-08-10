cfg_if! {
    if #[cfg(target_os = "freebsd")] {
        mod freebsd;
        pub use self::freebsd::*;
    } else {
        // ...
    }

}

