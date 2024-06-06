#!/bin/bash
set -eox pipefail

rm ./Cargo.lock

cargo clean

HOST_DIR="${HOST_DIR:-$(pwd)}"

docker run \
    --rm \
    --mount type=bind,source=$HOST_DIR,target=/host \
    --platform linux/amd64 \
    --cap-add=SYS_PTRACE \
    --security-opt seccomp=unconfined \
    -t nearprotocol/contract-builder:latest-amd64 \
    /bin/bash -c "cd /host && make build"
