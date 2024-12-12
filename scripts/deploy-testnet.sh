#!/bin/bash
set -eox pipefail

make dock

if [ $? -ne 0 ]; then
    echo ">> Error building contract"
    exit 1
fi

near deploy v9.jar.sweatty.testnet ./res/sweat_jar.wasm --force

near view v9.jar.sweatty.testnet contract_version
