#!/bin/bash
set -eox pipefail

# NOT `res/` — that holds the committed production artifacts; integration
# tests read from `res-integration/` (see tests/common/prepare.rs).
cargo near build non-reproducible-wasm --locked --out-dir res-integration --features integration-test --manifest-path contract/Cargo.toml
