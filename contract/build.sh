#!/bin/bash
set -eox pipefail

echo ">> Building contract"

rustup target add wasm32-unknown-unknown
cargo build --all --target wasm32-unknown-unknown --release

mv ./target/wasm32-unknown-unknown/release/sweat_jar.wasm ../res/sweat_jar.wasm
