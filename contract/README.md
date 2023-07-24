# SWEAT Jar Contract

A smart contract for staking of $SWEAT.

---

# Quickstart

1. Make sure you have installed [rust](https://rust.org/).
2. Install the [`NEAR CLI`](https://github.com/near/near-cli#setup)

<br />

## 1. Build and Deploy the Contract
First build the contract using provided build script:

```bash
./build.sh
```

Then deploy and initialize it. To deploy the contract to dev-account on Testnet use the following command:

```bash
near dev-deploy --wasmFile "res/sweat_jar.wasm" --initFunction "init" --initArgs '{"token_account_id": "ft.testnet", "fee_account_id": "fee.testnet", "admin_allowlist": ["admin.testnet"]}'
```

Once finished, check the `neardev/dev-account` file to find the address in which the contract was deployed:

```bash
cat ./neardev/dev-account
# e.g. dev-1659899566943-21539992274727
```

<br />
