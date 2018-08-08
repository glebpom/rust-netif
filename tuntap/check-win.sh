#!/bin/bash

set -e

docker build -t dhcp-win -f Dockerfile-win .
docker rm dhcp-win-container || true 
docker run -it --name=dhcp-win-container -v cargo_cache_win:/root/.cargo -v target_dhcp_win:/build/code/target dhcp-win 
