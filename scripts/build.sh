#!/bin/bash
set -eox pipefail

# --no-abi: legacy model types (e.g. numbers::U32) use hand-written serde
# and don't implement JsonSchema, so ABI generation can't run here.
cargo near build non-reproducible-wasm --no-abi --locked --out-dir res --manifest-path contract/Cargo.toml
