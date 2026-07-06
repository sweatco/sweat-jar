use std::path::PathBuf;

use anyhow::{anyhow, Result};
use near_workspaces::{network::Sandbox, types::NearToken, Account, Contract, Worker};

use super::{ft, jar, product::RegisterProductCommand};

const INITIAL_USER_BALANCE: NearToken = NearToken::from_near(10);
const ALICE_BOB_INITIAL_BALANCE: u128 = 100_000_000 * 10u128.pow(18);
const MANAGER_INITIAL_BALANCE: u128 = 100_000_000 * 10u128.pow(18);

const SWEAT_JAR_WASM_ENV: &str = "SWEAT_JAR_WASM";
const SWEAT_WASM_ENV: &str = "SWEAT_WASM";

/// A booted sandbox with the SWEAT token and the sweat_jar contract deployed and
/// wired together: `manager` is the jar's manager, `alice`/`bob` are funded and
/// registered with the token, `fee` is the jar's fee-collection account.
pub struct Context {
    // Held to keep the sandbox alive for the test's lifetime.
    pub worker: Worker<Sandbox>,
    pub jar: Contract,
    pub ft: Contract,
    pub manager: Account,
    pub alice: Account,
    pub bob: Account,
    pub fee: Account,
}

pub async fn prepare_contract(products: impl IntoIterator<Item = RegisterProductCommand>) -> Result<Context> {
    init_tracing();

    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;

    let ft = deploy(&worker, sweat_wasm_path(), "sweat", SWEAT_WASM_ENV).await?;
    let jar = deploy(&worker, sweat_jar_wasm_path(), "sweat_jar", SWEAT_JAR_WASM_ENV).await?;

    let manager = create_user(&root, "manager_longer_name_to_be_closer_to_real").await?;
    let alice = create_user(&root, "alice_longer_name_to_be_closer_to_real").await?;
    let bob = create_user(&root, "bob_longer_name_to_be_closer_to_real").await?;
    let fee = create_user(&root, "fee_longer_name_to_be_closer_to_real").await?;

    ft::new(&ft, ".u.sweat.testnet").await?;
    jar::init(&jar, ft.id(), fee.id(), manager.id(), root.id()).await?;

    ft::storage_deposit(&ft, jar.id()).await?;
    ft::storage_deposit(&ft, fee.id()).await?;
    ft::storage_deposit(&ft, alice.id()).await?;
    ft::storage_deposit(&ft, bob.id()).await?;

    ft::tge_mint(&ft, jar.id(), 100_000_000).await?;
    ft::tge_mint(&ft, alice.id(), ALICE_BOB_INITIAL_BALANCE).await?;
    ft::tge_mint(&ft, bob.id(), ALICE_BOB_INITIAL_BALANCE).await?;
    ft::tge_mint(&ft, manager.id(), MANAGER_INITIAL_BALANCE).await?;

    for product in products {
        jar::register_product(&jar, &manager, product.get()).await?;
    }

    Ok(Context {
        worker,
        jar,
        ft,
        manager,
        alice,
        bob,
        fee,
    })
}

impl Context {
    pub async fn fast_forward_minutes(&self, minutes: u64) -> Result<()> {
        fast_forward_minutes(&self.worker, minutes).await
    }

    pub async fn fast_forward_hours(&self, hours: u64) -> Result<()> {
        fast_forward_minutes(&self.worker, hours * 60).await
    }

    /// `bulk_create_jars` via the manager, minting enough SWEAT first (matches
    /// the previous nitka-era `ContextHelpers::bulk_create_jars` behavior).
    pub async fn bulk_create_jars(
        &self,
        account: &Account,
        product_id: &str,
        principal: u128,
        number_of_jars: u16,
    ) -> Result<()> {
        let total_amount = principal * number_of_jars as u128;

        ft::tge_mint(&self.ft, account.id(), 100_000_000_000).await?;

        let account_balance = ft::ft_balance_of(&self.ft, account.id()).await?;
        assert!(
            account_balance > total_amount,
            "Account doesn't have enough $SWEAT to create {number_of_jars} jars with {principal} principal. \
             Required: {total_amount} has: {account_balance}",
        );

        ft::ft_transfer(&self.ft, account, self.jar.id(), total_amount).await?;
        jar::bulk_create_jars(&self.jar, &self.manager, account.id(), product_id, principal, number_of_jars).await?;

        Ok(())
    }
}

pub fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,workspaces=warn,sandbox=warn,near_workspaces=warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

/// `Worker::fast_forward` (near-workspaces) advances the sandbox by a block-height
/// delta, not a real-time duration. This ratio is an empirical calibration for
/// this sandbox so `fast_forward_minutes` advances on-chain time by roughly the
/// requested number of minutes — confirmed against `tests/fast_forward.rs`.
const BLOCKS_PER_MINUTE: u64 = 240;

pub async fn fast_forward_minutes(worker: &Worker<Sandbox>, minutes: u64) -> Result<()> {
    worker.fast_forward(BLOCKS_PER_MINUTE * minutes).await?;
    Ok(())
}

fn wasm_path(env_var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(env_var).map(PathBuf::from).unwrap_or(default)
}

fn res_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("res").join(file)
}

fn sweat_wasm_path() -> PathBuf {
    wasm_path(SWEAT_WASM_ENV, res_path("sweat.wasm"))
}

fn sweat_jar_wasm_path() -> PathBuf {
    wasm_path(SWEAT_JAR_WASM_ENV, res_path("sweat_jar.wasm"))
}

async fn deploy(worker: &Worker<Sandbox>, path: PathBuf, label: &str, env_var: &str) -> Result<Contract> {
    let bytes = std::fs::read(&path).map_err(|e| {
        anyhow!(
            "failed to read {label} WASM at {} — did you run `make build-integration`? \
             Override the path with the {env_var} env var. ({e})",
            path.display()
        )
    })?;
    Ok(worker.dev_deploy(&bytes).await?)
}

async fn create_user(root: &Account, name: &str) -> Result<Account> {
    Ok(root
        .create_subaccount(name)
        .initial_balance(INITIAL_USER_BALANCE)
        .transact()
        .await?
        .into_result()?)
}
