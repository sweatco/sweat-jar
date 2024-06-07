#!/bin/bash
set -eox pipefail

#near deploy v8.jar.sweatty.testnet ./res/sweat_jar.wasm --initFunction migrate_state_to_near_sdk_5 --initArgs '{}'
near deploy v8.jar.sweatty.testnet ./res/sweat_jar.wasm

near view v8.jar.sweatty.testnet contract_version
