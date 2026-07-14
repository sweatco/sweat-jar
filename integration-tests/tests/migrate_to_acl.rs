//! Rehearses `migrate_state_to_acl`: deploys the *actual pre-ACL production
//! wasm* (a fixture copied from the last commit before this feature — see
//! `tests/fixtures/sweat_jar_pre_acl.wasm`), deploys the current wasm over it,
//! and calls the migration. Once this has been run against every live
//! deployment, delete this file, the fixture, and
//! `contract/src/migration/acl.rs`'s `migrate_state_to_acl`/`ContractBeforeAcl`.

mod common;

use std::path::PathBuf;

use anyhow::Result;
use common::{ft, jar, panic::PanicFinder, product::RegisterProductCommand};
use near_workspaces::{operations::Function, types::Gas};
use serde_json::json;

fn pre_acl_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sweat_jar_pre_acl.wasm")
}

fn current_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../res/sweat_jar.wasm")
}

#[tokio::test]
async fn migrate_state_to_acl() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;

    let pre_acl_wasm = std::fs::read(pre_acl_wasm_path())?;
    let current_wasm = std::fs::read(current_wasm_path())?;
    let sweat_wasm = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../res/sweat.wasm"))?;

    let ft = worker.dev_deploy(&sweat_wasm).await?;
    let jar = worker.dev_deploy(&pre_acl_wasm).await?;

    let old_manager = root
        .create_subaccount("old_manager")
        .initial_balance(near_workspaces::types::NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    let alice = root
        .create_subaccount("alice")
        .initial_balance(near_workspaces::types::NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    let new_super_admin = root
        .create_subaccount("new_super_admin")
        .initial_balance(near_workspaces::types::NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    let new_version = root
        .create_subaccount("new_version")
        .initial_balance(near_workspaces::types::NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;

    ft::new(&ft, ".u.sweat.testnet").await?;

    // Pre-ACL `init` signature: (token_account_id, fee_account_id, manager, new_version_account_id).
    jar.call("init")
        .args_json(json!({
            "token_account_id": ft.id(),
            "fee_account_id": old_manager.id(),
            "manager": old_manager.id(),
            "new_version_account_id": new_version.id(),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    ft::storage_deposit(&ft, jar.id()).await?;
    ft::storage_deposit(&ft, alice.id()).await?;
    ft::tge_mint(&ft, jar.id(), 100_000_000 * 10u128.pow(18)).await?;
    ft::tge_mint(&ft, alice.id(), 1_000_000).await?;

    let product = RegisterProductCommand::Locked12Months12Percents;
    old_manager
        .call(jar.id(), "register_product")
        .args_json(json!({ "command": product.json() }))
        .deposit(near_workspaces::types::NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let used = jar::used_amount(jar::create_jar(&ft, &jar, &alice, &product.id(), 500_000).await?)?;
    assert_eq!(used, 500_000);

    let products_before = jar::get_products(&jar).await?;
    let jars_before = jar::get_jars_for_account(&jar, alice.id()).await?;
    assert_eq!(products_before.len(), 1);
    assert_eq!(jars_before.len(), 1);

    // Deploy the new wasm and run the migration, batched in one transaction —
    // mirrors how this would actually be rolled out.
    let roles = jar::all_roles_to(new_super_admin.id());
    jar.as_account()
        .batch(jar.id())
        .deploy(&current_wasm)
        .call(
            Function::new("migrate_state_to_acl")
                .args_json(json!({ "super_admin": new_super_admin.id(), "roles": roles }))
                .gas(Gas::from_tgas(300)),
        )
        .transact()
        .await?
        .into_result()?;

    // Old state survived.
    let products_after = jar::get_products(&jar).await?;
    let jars_after = jar::get_jars_for_account(&jar, alice.id()).await?;
    assert_eq!(products_after, products_before);
    assert_eq!(jars_after, jars_before);

    // `new_super_admin` has every role (granted explicitly via `roles`).
    let has_role: bool = jar
        .view("acl_has_role")
        .args_json(json!({ "role": "ProductManager", "account_id": new_super_admin.id() }))
        .await?
        .json()?;
    assert!(has_role);

    // The old `manager` inherits nothing automatically — the caller decides
    // who gets roles via the `roles` argument, and `old_manager` wasn't in it.
    let result = old_manager
        .call(jar.id(), "set_enabled")
        .args_json(json!({ "product_id": product.id(), "is_enabled": false }))
        .deposit(near_workspaces::types::NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;
    assert!(result.has_panic("Insufficient permissions for method set_enabled"));

    // Running the migration again fails: state no longer matches the pre-ACL
    // shape the migration expects to read.
    let result = jar
        .as_account()
        .call(jar.id(), "migrate_state_to_acl")
        .args_json(json!({ "super_admin": new_super_admin.id(), "roles": roles }))
        .max_gas()
        .transact()
        .await?;
    assert!(result.is_failure());

    Ok(())
}
