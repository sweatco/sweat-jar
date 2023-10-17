#!/bin/bash
set -eox pipefail

if [ -z ${MEASURE_JARS_COUNT+x} ]; then echo "MEASURE_JARS_COUNT is unset"; else echo "MEASURE_JARS_COUNT is set to '$MEASURE_JARS_COUNT'"; fi
if [ -z ${MEASURE_CHUNK_SIZE+x} ]; then echo "MEASURE_CHUNK_SIZE is unset"; else echo "MEASURE_CHUNK_SIZE is set to '$MEASURE_CHUNK_SIZE'"; fi


rm -f measured.txt

cargo test --package integration-tests --lib measure::stake::measure_stake_total_test -- --ignored --exact
cargo test --package integration-tests --lib measure::restake::measure_restake_total_test -- --ignored --exact
cargo test --package integration-tests --lib measure::claim::measure_claim_total_test -- --ignored --exact
cargo test --package integration-tests --lib measure::withdraw::measure_withdraw_total_test -- --ignored --exact
