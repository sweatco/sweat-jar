#!/bin/bash
set -eox pipefail

echo ">> Building contract with coverage"

export RUSTC_BOOTSTRAP=1
export RUSTFLAGS="-Cinstrument-coverage -Zno-profiler-runtime -Zlocation-detail=none"

rustup target add wasm32-unknown-unknown

cargo build -p sweat_jar --target wasm32-unknown-unknown --profile=contract

cp ./target/wasm32-unknown-unknown/contract/sweat_jar.wasm res/sweat_jar_coverage.wasm
