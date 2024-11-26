#!/bin/bash
set -eox pipefail

rustup component add clippy

cargo clippy -p sweat_jar \
    -- \
    \
    -W clippy::all \
    -W clippy::pedantic \
    \
    -A clippy::module_name_repetitions \
    -A clippy::module_inception \
    -A clippy::needless-pass-by-value \
    -A clippy::must-use-candidate \
    -A clippy::missing_panics_doc \
    -A clippy::explicit_deref_methods \
    \
    -D warnings
