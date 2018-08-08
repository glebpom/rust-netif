ifstructs
=========

A Rust library with native bindings to unix `if` structures (for now only `ifreq`)

[![Build Status](https://travis-ci.org/glebpom/ifstructs.svg?branch=master)](https://travis-ci.org/glebpom/ifstructs)
[![Latest version](https://img.shields.io/crates/v/ifstructs.svg)](https://crates.io/crates/ifstructs)
[![Documentation](https://docs.rs/ifstructs/badge.svg)](https://docs.rs/ifstructs)
![License](https://img.shields.io/crates/i/ifstructs.svg)


## Usage

First, add the following to your `Cargo.toml`:

```toml
[dependencies]
ifstructs = "0.0.1"
```

Next, add this to your crate:

```rust
extern crate ifstructs;

use ifstructs::ifreq;

fn main() {
  let mut req = ifreq::from_name("eth0").unwrap();
  
  ...
}

```
