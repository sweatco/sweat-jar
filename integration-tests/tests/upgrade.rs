//! Exercises near-plugins' `Upgradable` (`up_stage_code` / `up_deploy_code`):
//! that staging and deploying are gated by distinct roles (`StagingManager`
//! vs `UpgradeManager`), and that a real redeploy preserves contract state
//! and leaves the contract functional.

mod common;

use std::path::PathBuf;

use anyhow::Result;
use common::{jar, panic::PanicFinder, prepare::prepare_contract, product::RegisterProductCommand};
use serde_json::json;

fn insufficient_permissions(method: &str) -> String {
    format!("Insufficient permissions for method {method} restricted by access control.")
}

fn jar_wasm_bytes() -> Result<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../res-integration/sweat_jar.wasm");
    Ok(std::fs::read(&path).map_err(|e| {
        anyhow::anyhow!(
            "failed to read sweat_jar wasm at {} — did you run `make build-integration`? ({e})",
            path.display()
        )
    })?)
}

#[tokio::test]
async fn upgrade_access_control() -> Result<()> {
    let context = prepare_contract([]).await?;
    let nobody = &context.bob;
    let code = jar_wasm_bytes()?;

    // Neither role: both staging and deploying are rejected.
    let result = jar::up_stage_code(&context.jar, nobody, code.clone()).await?;
    assert!(result.has_panic(&insufficient_permissions("up_stage_code")));
    assert!(jar::up_staged_code_hash(&context.jar).await?.is_none());

    let result = jar::up_deploy_code(&context.jar, nobody, "irrelevant").await?;
    assert!(result.has_panic(&insufficient_permissions("up_deploy_code")));

    // Grant only StagingManager: can stage, still can't deploy.
    context
        .manager
        .call(context.jar.id(), "acl_grant_role")
        .args_json(json!({ "role": "StagingManager", "account_id": nobody.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    jar::up_stage_code(&context.jar, nobody, code).await?.into_result()?;
    let hash = jar::up_staged_code_hash(&context.jar).await?.expect("code was staged");

    let result = jar::up_deploy_code(&context.jar, nobody, &hash).await?;
    assert!(result.has_panic(&insufficient_permissions("up_deploy_code")));

    Ok(())
}

#[tokio::test]
async fn upgrade_deploy_round_trip() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;
    let alice = &context.alice;
    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    jar::create_jar(&context.ft, &context.jar, alice, &product_id, 1_000_000)
        .await?
        .into_result()?;
    let jars_before = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    assert_eq!(jars_before.len(), 1);

    // `manager` holds every role (all_roles_to), including StagingManager/UpgradeManager.
    let code = jar_wasm_bytes()?;
    jar::up_stage_code(&context.jar, &context.manager, code)
        .await?
        .into_result()?;
    let hash = jar::up_staged_code_hash(&context.jar).await?.expect("code was staged");

    jar::up_deploy_code(&context.jar, &context.manager, &hash)
        .await?
        .into_result()?;

    // Pre-upgrade state survived the redeploy.
    let jars_after = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    assert_eq!(jars_after, jars_before);

    // The upgraded contract is still functional.
    jar::create_jar(&context.ft, &context.jar, alice, &product_id, 500_000)
        .await?
        .into_result()?;
    let jars_final = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    assert_eq!(jars_final.len(), 2);

    Ok(())
}
