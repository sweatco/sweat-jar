#!/bin/bash
set -eox pipefail

echo ">> Mutation tests"

cargo install --locked cargo-mutants
cargo mutants
