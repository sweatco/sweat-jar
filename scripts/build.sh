#!/bin/bash
set -eox pipefail

cargo near build non-reproducible-wasm --locked --out-dir res --manifest-path contract/Cargo.toml
