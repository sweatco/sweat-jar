#!/bin/bash
set -eox pipefail

rm -f measured.txt

cargo test --package integration-tests --lib measure::stake::measure_stake_total_test -- --ignored --exact
cargo test --package integration-tests --lib measure::restake::measure_restake_total_test -- --ignored --exact
cargo test --package integration-tests --lib measure::claim::measure_claim_total_test -- --ignored --exact
cargo test --package integration-tests --lib measure::withdraw::measure_withdraw_total_test -- --ignored --exact
