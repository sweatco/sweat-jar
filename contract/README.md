# SWEAT DeFi Jar Contract

A smart contract for staking of $SWEAT.

---

# Quickstart

1. Make sure you have installed [rust](https://rust.org/).
2. Install the [`NEAR CLI`](https://github.com/near/near-cli#setup)

If you already have `rustup` installed, you can ensure that the correct version of the compiler and the NEAR CLI is installed as well: 

```shell
make install
```

## Build and Deploy the Contract
First build the contract using provided `make` command:

```bash
make build
```

Then deploy and initialize it. Rename `dev.env.example` to `dev.env` and define variable values there. To deploy the contract to dev-account on Testnet use the following command:

```bash
make deploy
```

Once finished, check the `neardev/dev-account` file to find the address in which the contract was deployed:

```bash
cat ./neardev/dev-account
# e.g. dev-1659899566943-21539992274727
```

## Reproducible build

If you build your contract on two different machines, it's highly likely that you'll obtain two binaries that are 
similar but not exactly identical. Your build outcome can be influenced by various factors in your build environment, 
such as the locale, timezone, build path, and numerous other variables.

To obtain an identical build artifact on any machine, matching the one deployed on NEAR, you can build it using Docker:

```shell
make build-in-docker
```
