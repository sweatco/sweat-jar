#!/bin/bash
set -eox pipefail

echo ">> Building contract"

rustup target add wasm32-unknown-unknown
cargo build -p sweat_jar --target wasm32-unknown-unknown --profile=contract --features integration-test,

cp ./target/wasm32-unknown-unknown/contract/sweat_jar.wasm res/sweat_jar.wasm
