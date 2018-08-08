#!/bin/bash
# IP=192.168.17.205
IP=192.168.71.25
rsync -vr --exclude .history --exclude target ./ root@${IP}:/root/rust-tuntap
scp -r src examples root@${IP}:/root/rust-tuntap
ssh -t root@${IP} "/usr/local/bin/bash -c 'cd /root/rust-tuntap && RUST_BACKTRACE=full /root/.cargo/bin/cargo run --example test'"