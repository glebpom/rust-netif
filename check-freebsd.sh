#!/bin/bash
# IP=192.168.17.178
IP=192.168.71.25
rsync -vr --exclude .history --exclude .git --exclude target ./ root@${IP}:/root/netif
scp -r ifcontrol/src ifstructs/src root@${IP}:/root/netif
ssh -t root@${IP} "/usr/local/bin/bash -c 'cd /root/netif && RUST_BACKTRACE=full /root/.cargo/bin/cargo test -p ifcontrol impls:: -- --nocapture'"