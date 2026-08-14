use serde_json::json;
use sweat_jar::Roles;

mod common;
use common::{
    jar,
    panic::PanicFinder,
    prepare::{jar_wasm_bytes, prepare_contract},
    product::RegisterProductCommand,
};

fn insufficient_permissions(method: &str) -> String {
    format!("Insufficient permissions for method {method} restricted by access control.")
}

/// `up_stage_code` is restricted to `StagingManager` and `up_deploy_code` to
/// `UpgradeManager`. The two roles are distinct: holding the stager role does
/// not grant the deployer one, and an account with neither role can do neither.
#[tokio::test]
#[tracing::instrument]
async fn upgrade_access_control() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    let context = prepare_contract([]).await?;
    let code = jar_wasm_bytes()?;

    // Neither role: both methods rejected.
    let result = context
        .alice
        .call(context.jar.id(), "up_stage_code")
        .args(code.clone())
        .max_gas()
        .transact()
        .await?;
    assert!(result.into_result().has_panic(&insufficient_permissions("up_stage_code")));

    let staged: Option<String> = context.jar.view("up_staged_code_hash").await?.json()?;
    assert_eq!(staged, None, "unauthorized staging must not store any code");

    let result = context
        .alice
        .call(context.jar.id(), "up_deploy_code")
        .args_json(json!({ "hash": "ignored", "function_call_args": null }))
        .max_gas()
        .transact()
        .await?;
    assert!(result.into_result().has_panic(&insufficient_permissions("up_deploy_code")));

    // StagingManager only: can stage, still cannot deploy. The grant must be
    // signed by `manager` (the super-admin) — the jar contract account holds
    // no admin power after init.
    context
        .manager
        .call(context.jar.id(), "acl_grant_role")
        .args_json(json!({ "role": String::from(Roles::StagingManager), "account_id": context.alice.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    context
        .alice
        .call(context.jar.id(), "up_stage_code")
        .args(code.clone())
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let staged: Option<String> = context.jar.view("up_staged_code_hash").await?.json()?;
    assert!(staged.is_some(), "staged code hash should be set after staging");

    let result = context
        .alice
        .call(context.jar.id(), "up_deploy_code")
        .args_json(json!({ "hash": staged.unwrap(), "function_call_args": null }))
        .max_gas()
        .transact()
        .await?;
    assert!(
        result.into_result().has_panic(&insufficient_permissions("up_deploy_code")),
        "the stager role must not grant deploy permission"
    );

    Ok(())
}

/// Full stage → deploy flow: an `UpgradeManager` re-deploys the contract's own
/// current code over itself, and existing state (a jar created before the
/// upgrade) survives.
#[tokio::test]
#[tracing::instrument]
async fn upgrade_deploy_round_trip() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    let context = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;
    let code = jar_wasm_bytes()?;
    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    jar::create_jar(&context.jar, &context.ft, &context.alice, &product_id, 1_000_000).await?;
    let jars_before = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(
        1,
        jars_before.get_total_deposits_number(),
        "there should be a jar before the upgrade"
    );

    // `manager` already holds `StagingManager`/`UpgradeManager` from
    // `prepare_contract`'s init-time role grants (PROD-3696), so no explicit
    // grant is needed here — unlike the pre-PROD-3696 code, which granted
    // only 4 roles at `prepare_contract` time and needed this loop to add the
    // 2 upgrade roles afterward.

    context
        .manager
        .call(context.jar.id(), "up_stage_code")
        .args(code)
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let staged_hash: Option<String> = context.jar.view("up_staged_code_hash").await?.json()?;
    let staged_hash = staged_hash.expect("code must be staged before deploy");

    let result = context
        .manager
        .call(context.jar.id(), "up_deploy_code")
        .args_json(json!({ "hash": staged_hash, "function_call_args": null }))
        .max_gas()
        .transact()
        .await?;
    assert!(result.into_result().is_ok(), "deploy should succeed");

    let jars_after = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(
        1,
        jars_after.get_total_deposits_number(),
        "the jar created before the upgrade must survive it"
    );

    jar::create_jar(&context.jar, &context.ft, &context.alice, &product_id, 1_000_000).await?;
    let jars_final = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(
        2,
        jars_final.get_total_deposits_number(),
        "the upgraded contract should still accept new jars"
    );

    Ok(())
}
