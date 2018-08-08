use ifstructs;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }

    foreign_links {
        Nix(::nix::Error) #[cfg(unix)];
        Io(::std::io::Error);
    }

    errors {
        IfaceNotFound {
            description("iface not found")
            display("iface not found")
        }

    }
}
