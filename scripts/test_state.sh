#!/bin/bash
set -eox pipefail

echo "Deploying FT contract"

#near deploy vladas_ft.testnet --wasmFile "res/sweat.wasm" --initFunction "new" --initArgs "{\"postfix\": \".u.sweat.testnet\" }"
#near deploy vladas_jar.testnet --wasmFile "res/sweat_jar.wasm" --initFunction "init" --initArgs "{\"token_account_id\": \"vladas_ft.testnet\", \"manager\": \"vladas_ft.testnet\", \"fee_account_id\": \"vladas_ft.testnet\"}"

near deploy vladas_ft.testnet --wasmFile "res/sweat.wasm"


#near state vladas_jar.testnet
#main hash: 2E7HiUydH3odwPMYCKpF9NrocB8YFRU79rnc7mBHTSvi

near deploy vladas_jar.testnet --wasmFile "res/sweat_jar.wasm"
near deploy vladas_jar.testnet --wasmFile "res/sweat_jar_main_reset.wasm" #AowFLGnYVP9eTGn8Yjj9NSA4SaxQHVan3bVgzBRsH5CZ



