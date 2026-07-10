use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use near_workspaces::{operations::Function, types::NearToken, DevNetwork, Worker};
use serde_json::json;
use sweat_jar::{all_roles_to, Roles};

mod common;
use common::{ft, jar, prepare::jar_wasm_bytes, product::RegisterProductCommand};

/// The wasm binary as built from commit `2f8c645` (the tip of PROD-3638, the
/// last commit before PROD-3639 replaced the `manager: AccountId` field with
/// role-based ACL).
const PRE_ACL_WASM: &[u8] = include_bytes!("../fixtures/sweat_jar_pre_acl_v4.1.1.wasm");

/// The exact binary running on `v2.jars.sweat` (mainnet), fetched via
/// `view_code` — sha256 `aba15bc7…`, matching the on-chain code hash
/// `CYyRWesovLcRtLrv81PN6ujJ9g26h2hbu6NUFVAjsKkA` and the `res/sweat_jar.wasm`
/// committed at `b91847f` (source rev `9ab8189`, v4.1.1, near-sdk 5.14).
/// Migrating from this fixture rehearses the real mainnet upgrade.
const MAINNET_WASM: &[u8] = include_bytes!("../fixtures/sweat_jar_mainnet_v4_1_1_9ab8189.wasm");

/// Big enough that 12% APY accrues nonzero interest within a few seconds of
/// block time — the rehearsal must also work on live testnet, where there is
/// no `fast_forward`.
const PRINCIPAL: u128 = 10u128.pow(24);

fn sweat_ft_wasm() -> Result<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../res/sweat.wasm");
    Ok(std::fs::read(path)?)
}

/// Deploys a real pre-PROD-3639 wasm, initializes it with the old 4-arg
/// `init(token_account_id, fee_account_id, manager, previous_version_account_id)`
/// signature, registers a product and creates a funded account with a jar —
/// real state written by the OLD binary — then deploys the new code and calls
/// `migrate` in the same batched transaction (signed by the contract's own
/// account, matching `#[private]`). Verifies afterwards that the account
/// written by the old version reads back intact, that roles/super-admin are
/// provisioned, and that `claim_total` pays accrued interest out to the FT
/// balance.
///
/// Generic over the worker so the same scenario runs in the local sandbox or
/// on live testnet (`near_workspaces::testnet()`), where accounts are dev
/// accounts and time can't be fast-forwarded.
async fn run_migration_rehearsal(worker: &Worker<impl DevNetwork + 'static>, old_wasm: &[u8], new_wasm: &[u8]) -> Result<()> {
    common::prepare::init_tracing();

    let jar_contract = worker.dev_deploy(old_wasm).await?;
    let ft = worker.dev_deploy(&sweat_ft_wasm()?).await?;

    let fee = worker.dev_create_account().await?;
    let manager = worker.dev_create_account().await?;
    let legacy = worker.dev_create_account().await?;
    let new_super_admin = worker.dev_create_account().await?;
    let operator = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    ft::new(&ft, ".u.sweat.testnet").await?;

    jar_contract
        .call("init")
        .args_json(json!({
            "token_account_id": ft.id(),
            "fee_account_id": fee.id(),
            "manager": manager.id(),
            "previous_version_account_id": legacy.id(),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    ft::storage_deposit(&ft, jar_contract.id()).await?;
    ft::storage_deposit(&ft, alice.id()).await?;
    ft::tge_mint(&ft, jar_contract.id(), PRINCIPAL).await?;
    ft::tge_mint(&ft, alice.id(), 10 * PRINCIPAL).await?;

    let product = RegisterProductCommand::Locked12Months12Percents.get();
    manager
        .call(jar_contract.id(), "register_product")
        .args_json(json!({ "product": product }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let products_before = jar::get_products(&jar_contract).await?;
    assert_eq!(1, products_before.len(), "product should exist before migration");

    // Real account state written by the OLD binary.
    jar::create_jar(
        &jar_contract,
        &ft,
        &alice,
        &RegisterProductCommand::Locked12Months12Percents.id(),
        PRINCIPAL,
    )
    .await?;
    let principal_before = jar::get_jars_for_account(&jar_contract, alice.id())
        .await?
        .get_total_principal();
    assert_eq!(PRINCIPAL, principal_before, "jar must exist before migration");

    jar_contract
        .batch()
        .deploy(new_wasm)
        .call(
            Function::new("migrate")
                .args_json(json!({
                    "super_admin": new_super_admin.id(),
                    "roles": all_roles_to(operator.id()),
                }))
                .max_gas(),
        )
        .transact()
        .await?
        .into_result()?;

    // Products written by the old version survive.
    let products_after = jar::get_products(&jar_contract).await?;
    assert_eq!(1, products_after.len(), "product registered before migration must survive it");
    assert_eq!(products_before[0].id, products_after[0].id);

    // Accounts written by the old version read back intact.
    let jars_after = jar::get_jars_for_account(&jar_contract, alice.id()).await?;
    assert_eq!(
        PRINCIPAL,
        jars_after.get_total_principal(),
        "account written by the old version must read back with its principal after migration"
    );

    // ACL is provisioned as requested.
    for role in Roles::all() {
        let role = String::from(role);
        let has_role: bool = jar_contract
            .view("acl_has_role")
            .args_json(json!({ "role": role, "account_id": operator.id() }))
            .await?
            .json()?;
        assert!(has_role, "operator should hold role {role} after migration");
    }

    let is_super_admin: bool = jar_contract
        .view("acl_is_super_admin")
        .args_json(json!({ "account_id": new_super_admin.id() }))
        .await?
        .json()?;
    assert!(is_super_admin, "new_super_admin should be super-admin after migration");

    let old_manager_is_super_admin: bool = jar_contract
        .view("acl_is_super_admin")
        .args_json(json!({ "account_id": manager.id() }))
        .await?
        .json()?;
    assert!(
        !old_manager_is_super_admin,
        "the old manager must not automatically become super-admin"
    );

    // Claim works after migration. Wait for nonzero interest — no
    // fast_forward on live networks, so nudge block production with no-op
    // transfers and poll.
    let mut interest = 0;
    for _ in 0..30 {
        interest = jar::get_total_interest(&jar_contract, alice.id())
            .await?
            .amount
            .total
            .0;
        if interest > 0 {
            break;
        }
        alice
            .transfer_near(fee.id(), NearToken::from_yoctonear(1))
            .await?
            .into_result()?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    assert!(interest > 0, "interest must accrue on the migrated account");

    let balance_before = ft::ft_balance_of(&ft, alice.id()).await?;
    let claimed = jar::claim_total(&jar_contract, &alice, None).await?.get_total().0;
    assert!(claimed > 0, "claim after migration must pay out");

    let balance_after = ft::ft_balance_of(&ft, alice.id()).await?;
    assert_eq!(
        balance_before + claimed,
        balance_after,
        "claimed interest must arrive on the FT balance"
    );

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn migrate_from_real_pre_acl_deployment() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    run_migration_rehearsal(&worker, PRE_ACL_WASM, &jar_wasm_bytes()?).await
}

#[tokio::test]
#[tracing::instrument]
async fn migrate_from_exact_mainnet_binary() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    run_migration_rehearsal(&worker, MAINNET_WASM, &jar_wasm_bytes()?).await
}

/// Live-binaries rehearsal: fetches the binary currently RELEASED on mainnet
/// (`v2.jars.sweat`) and the new build currently DEPLOYED on testnet
/// (`v11.jar.sweatty.testnet`) via `view_code`, then runs the released -> new
/// migration in the local sandbox. `#[ignore]`d so plain `make integration`
/// stays hermetic; run explicitly with
/// `cargo test --test migrate -- --ignored`.
#[tokio::test]
#[ignore]
#[tracing::instrument]
async fn migrate_from_released_mainnet_to_deployed_testnet_build() -> Result<()> {
    let mainnet = near_workspaces::mainnet().await?;
    let released_wasm = mainnet.view_code(&"v2.jars.sweat".parse()?).await?;

    let testnet = near_workspaces::testnet().await?;
    let new_wasm = testnet.view_code(&"v11.jar.sweatty.testnet".parse()?).await?;

    let worker = near_workspaces::sandbox().await?;
    run_migration_rehearsal(&worker, &released_wasm, &new_wasm).await
}

/// The same rehearsal executed ON LIVE TESTNET: dev accounts are created on
/// the real network, the released mainnet binary is deployed to one, and the
/// migration to the build currently on `v11.jar.sweatty.testnet` runs against
/// real infrastructure. Slower and spends faucet balance — run explicitly
/// with `cargo test --test migrate -- --ignored on_live_testnet`.
#[tokio::test]
#[ignore]
#[tracing::instrument]
async fn migrate_on_live_testnet_from_released_to_deployed_build() -> Result<()> {
    let mainnet = near_workspaces::mainnet().await?;
    let released_wasm = mainnet.view_code(&"v2.jars.sweat".parse()?).await?;

    let testnet = near_workspaces::testnet().await?;
    let new_wasm = testnet.view_code(&"v11.jar.sweatty.testnet".parse()?).await?;

    run_migration_rehearsal(&testnet, &released_wasm, &new_wasm).await
}
