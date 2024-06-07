#!/bin/bash
set -eox pipefail

near deploy jars.sweat ./res/sweat_jar.wasm --initFunction migrate_state_to_near_sdk_5 --initArgs '{}' --networkId mainnet
#near deploy jars.sweat ./res/sweat_jar.wasm --networkId mainnet

near view jars.sweat contract_version --networkId mainnet
