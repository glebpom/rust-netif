#!/bin/bash

set -e

docker build -t netif-linux -f Dockerfile-linux .
docker rm netif-linux-container || true 
docker run -it --privileged --name=netif-linux-container -v cargo_cache_linux:/root/.cargo -v target_dhcp_linux:/build/code/target netif-linux
