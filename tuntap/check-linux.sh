#!/bin/bash

set -e

# docker build -t dhcp-win -f Dockerfile-win .
docker build -t dhcp-linux -f Dockerfile-linux .

# docker rm dhcp-win-container || true 
docker rm dhcp-linux-container || true 

# docker run -it --name=dhcp-win-container -v cargo_cache_win:/root/.cargo -v target_dhcp_win:/build/code/target dhcp-win 
docker run -it --privileged --name=dhcp-linux-container -v cargo_cache_linux:/root/.cargo -v target_dhcp_linux:/build/code/target dhcp-linux

# cargo check 