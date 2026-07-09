#!/bin/bash
set -eox pipefail

# Deliberately NOT `res/`: that directory holds the committed production
# artifacts, and the integration-test-featured wasm must never end up there.
# Integration tests read from `res-integration/` (see tests/common/prepare.rs).
cargo near build non-reproducible-wasm --locked --out-dir res-integration --features integration-test --manifest-path contract/Cargo.toml
