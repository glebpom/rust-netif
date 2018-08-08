macro_rules! set_name {
    ($name_field:expr, $name_str:expr) => {{
        let name_c = &CString::new($name_str.to_owned())
            .map_err(|_| Error::from(ErrorKind::BadData("malformed interface name".to_owned())))?;
        let name_slice = name_c.as_bytes_with_nul();
        if name_slice.len() > IFNAMSIZ {
            bail!(ErrorKind::NameTooLong(name_slice.len(), IFNAMSIZ));
        }
        $name_field[..name_slice.len()].clone_from_slice(name_slice);

        Ok(())
    }};
}

macro_rules! get_name {
    ($name_field:expr) => {{
        let nul_pos = match $name_field.iter().position(|x| *x == 0) {
            Some(p) => p,
            None => bail!(ErrorKind::BadData("malformed interface name".to_owned())),
        };

        CString::new(&$name_field[..nul_pos])
            .unwrap()
            .into_string()
            .map_err(|_| ErrorKind::BadData("malformed interface name".to_owned()).into())
    }};
}
