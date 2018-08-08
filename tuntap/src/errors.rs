use ifcontrol;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        Ifcontrol(ifcontrol::Error, ifcontrol::ErrorKind);
    }

    foreign_links {
        Nix(::nix::Error) #[cfg(unix)];
        Io(::std::io::Error);
    }

    errors {
        NotFound(msg: String) {
            description("not found")
            display("not found: '{}'", msg)
        }

        MaxNumberReached(max: usize) {
            description("max number of virtual interfaces reached")
            display("max number of virtual interfaces reached: '{}'", max)
        }

        NameTooLong(s: usize, max: usize) {
            description("name too long")
            display("name too long: {} while max is {}", s, max)
        }

        BadArguments(msg: String) {
            description("bad arguments")
            display("bad arguments: '{}'", msg)
        }

        NotSupported(msg: String) {
            description("backend is not supported")
            display("backend is not supported: '{}'", msg)
        }

        DriverNotFound(msg: String) {
            description("driver not found")
            display("driver not found: '{}'", msg)
        }

        BadData(msg: String) {
            description("bad data received")
            display("bad data received: '{}'", msg)
        }

        Busy {
            description("device busy")
            display("device busy")
        }

        Other(msg: String) {
            description("other error")
            display("other error: '{}'", msg)
        }

    }
}
