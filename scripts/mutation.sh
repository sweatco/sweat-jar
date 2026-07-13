#!/bin/bash
set -eox pipefail

echo ">> Mutation tests"

cargo install --locked cargo-mutants@26.0.0
cargo mutants -p sweat_jar
