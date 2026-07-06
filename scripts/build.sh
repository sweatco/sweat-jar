#!/bin/bash
set -eox pipefail

rustup target add wasm32-unknown-unknown
cargo near build non-reproducible-wasm --locked --out-dir res --manifest-path contract/Cargo.toml
