#!/bin/bash
set -eox pipefail

echo ">> Building contract with coverage"

export RUSTC_BOOTSTRAP=1
export RUSTFLAGS="-Cinstrument-coverage -Zno-profiler-runtime -Zlocation-detail=none --emit=llvm-ir"

rustup target add wasm32-unknown-unknown

cargo build -p sweat_jar --target wasm32-unknown-unknown --profile=coverage

cp ./target/wasm32-unknown-unknown/coverage/sweat_jar.wasm res/sweat_jar_coverage.wasm
cp ./target/wasm32-unknown-unknown/coverage/deps/sweat_jar.ll res/sweat_jar_coverage.ll

perl -i -p0e 's/(^define[^\n]*\n).*?^}\s*$/$1start:\n  unreachable\n}\n/gms' res/sweat_jar_coverage.ll

clang-17 res/sweat_jar_coverage.ll -o res/sweat_jar_coverage.o -Wno-override-module -c
