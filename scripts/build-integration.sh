#!/bin/bash
set -eox pipefail

cargo near build non-reproducible-wasm --locked --out-dir res --features integration-test --manifest-path contract/Cargo.toml
