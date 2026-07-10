use anyhow::Result;
use near_workspaces::{operations::Function, types::NearToken};
use serde_json::json;
use sweat_jar::{all_roles_to, Roles};

mod common;
use common::{prepare::jar_wasm_bytes, product::RegisterProductCommand};

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

async fn create_user(root: &near_workspaces::Account, name: &str) -> Result<near_workspaces::Account> {
    Ok(root
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?)
}

/// Deploys a real pre-PROD-3639 wasm, initializes it with the old 4-arg
/// `init(token_account_id, fee_account_id, manager, previous_version_account_id)`
/// signature, registers a product to give the contract real state to lose,
/// then deploys the current code and calls `migrate` in the same batched
/// transaction (signed by the contract's own account, matching `#[private]`)
/// — proving both that a real binary-to-binary migration succeeds and that
/// pre-migration state and roles come out the other side correctly.
async fn run_migration_rehearsal(old_wasm: &[u8]) -> Result<()> {
    common::prepare::init_tracing();

    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;

    let jar = worker.dev_deploy(old_wasm).await?;

    let token = create_user(&root, "token_longer_name_to_be_closer_to_real").await?;
    let fee = create_user(&root, "fee_longer_name_to_be_closer_to_real").await?;
    let manager = create_user(&root, "manager_longer_name_to_be_closer_to_real").await?;
    let legacy = create_user(&root, "legacy_longer_name_to_be_closer_to_real").await?;
    let new_super_admin = create_user(&root, "super_admin_longer_name_to_be_closer_to_real").await?;
    let operator = create_user(&root, "operator_longer_name_to_be_closer_to_real").await?;

    jar.call("init")
        .args_json(json!({
            "token_account_id": token.id(),
            "fee_account_id": fee.id(),
            "manager": manager.id(),
            "previous_version_account_id": legacy.id(),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let product = RegisterProductCommand::Locked12Months12Percents.get();
    manager
        .call(jar.id(), "register_product")
        .args_json(json!({ "product": product }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let products_before = common::jar::get_products(&jar).await?;
    assert_eq!(1, products_before.len(), "product should exist before migration");
    assert_eq!(RegisterProductCommand::Locked12Months12Percents.id(), products_before[0].id);

    let current_wasm = jar_wasm_bytes()?;

    jar.batch()
        .deploy(&current_wasm)
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

    let products_after = common::jar::get_products(&jar).await?;
    assert_eq!(1, products_after.len(), "product registered before migration must survive it");
    assert_eq!(products_before[0].id, products_after[0].id);

    for role in Roles::all() {
        let role = String::from(role);
        let has_role: bool = jar
            .view("acl_has_role")
            .args_json(json!({ "role": role, "account_id": operator.id() }))
            .await?
            .json()?;
        assert!(has_role, "operator should hold role {role} after migration");
    }

    let is_super_admin: bool = jar
        .view("acl_is_super_admin")
        .args_json(json!({ "account_id": new_super_admin.id() }))
        .await?
        .json()?;
    assert!(is_super_admin, "new_super_admin should be super-admin after migration");

    let old_manager_is_super_admin: bool = jar
        .view("acl_is_super_admin")
        .args_json(json!({ "account_id": manager.id() }))
        .await?
        .json()?;
    assert!(
        !old_manager_is_super_admin,
        "the old manager must not automatically become super-admin"
    );

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn migrate_from_real_pre_acl_deployment() -> Result<()> {
    run_migration_rehearsal(PRE_ACL_WASM).await
}

#[tokio::test]
#[tracing::instrument]
async fn migrate_from_exact_mainnet_binary() -> Result<()> {
    run_migration_rehearsal(MAINNET_WASM).await
}
