#!/bin/bash
set -eox pipefail

cargo near build reproducible-wasm --out-dir res --manifest-path contract/Cargo.toml
