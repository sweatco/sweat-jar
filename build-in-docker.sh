#!/bin/bash
set -eox pipefail

HOST_DIR="${HOST_DIR:-$(pwd)}"

docker run \
     --mount type=bind,source=$HOST_DIR,target=/host \
     --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
     -t nearprotocol/contract-builder:latest-amd64 \
     /bin/bash -c "cd /host && ./build.sh"
