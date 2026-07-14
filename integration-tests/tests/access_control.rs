//! Verifies each `#[access_control_any(roles(...))]` gate (and the two manual
//! `acl_has_any_role` checks in `ft_receiver`/`internal`) actually rejects a
//! caller without the role, with the role-specific message near-plugins (or
//! the manual check mirroring it) produces. The "correct role succeeds" side
//! is already exercised end-to-end by the other integration tests (they all
//! call these methods as `manager`, who holds every role) — this file is
//! about proving the *rejection* is wired to the right role per method.

mod common;

use anyhow::Result;
use common::{ft, jar, panic::PanicFinder, prepare::prepare_contract, product::RegisterProductCommand};
use near_workspaces::types::NearToken;
use serde_json::json;
use sweat_jar_model::{jar::JarIdView, U32};

fn insufficient_permissions(method: &str) -> String {
    format!("Insufficient permissions for method {method} restricted by access control.")
}

#[tokio::test]
async fn product_manager_gate() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;
    let nobody = &context.bob;
    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    let result = nobody
        .call(context.jar.id(), "register_product")
        .args_json(json!({ "command": RegisterProductCommand::Locked12Months12Percents.json() }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;
    assert!(result.has_panic(&insufficient_permissions("register_product")));

    let result = nobody
        .call(context.jar.id(), "set_enabled")
        .args_json(json!({ "product_id": product_id, "is_enabled": false }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;
    assert!(result.has_panic(&insufficient_permissions("set_enabled")));

    let result = nobody
        .call(context.jar.id(), "set_public_key")
        .args_json(json!({ "product_id": product_id, "public_key": "aGVsbG8=" }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;
    assert!(result.has_panic(&insufficient_permissions("set_public_key")));

    Ok(())
}

#[tokio::test]
async fn oracle_gate() -> Result<()> {
    let context = prepare_contract([]).await?;
    let nobody = &context.bob;

    let result = jar::record_score(&context.jar, nobody, vec![(context.alice.id(), vec![(100, 0.into())])]).await?;
    assert!(result.has_panic(&insufficient_permissions("record_score")));

    Ok(())
}

#[tokio::test]
async fn maintainer_gate() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked10Minutes6Percents]).await?;
    let nobody = &context.bob;
    let fake_jar_id: JarIdView = U32(0);

    let result = jar::set_penalty(&context.jar, nobody, context.alice.id(), fake_jar_id, true).await?;
    assert!(result.has_panic(&insufficient_permissions("set_penalty")));

    let result = jar::batch_set_penalty(
        &context.jar,
        nobody,
        vec![(context.alice.id(), vec![fake_jar_id])],
        true,
    )
    .await?;
    assert!(result.has_panic(&insufficient_permissions("batch_set_penalty")));

    let result = jar::unlock_jars_for_account(&context.jar, nobody, context.alice.id()).await?;
    assert!(result.has_panic(&insufficient_permissions("unlock_jars_for_account")));

    let result = jar::force_migrate_account(&context.jar, nobody, context.alice.id()).await?;
    assert!(result.has_panic(&insufficient_permissions("force_migrate_account")));

    let result = jar::unlock_account(&context.jar, nobody, context.alice.id()).await?;
    assert!(result.has_panic(&insufficient_permissions("unlock_account")));

    let result = jar::migrate_products(&context.jar, nobody).await?;
    assert!(result.has_panic(&insufficient_permissions("migrate_products")));

    let result = jar::bulk_create_jars(&context.jar, nobody, context.alice.id(), "nonexistent", 1, 1).await;
    assert!(result.is_err());

    // Manual check (ft_on_transfer isn't a `#[near_bindgen]` method, so it can't
    // carry `#[access_control_any]`): migrating CeFi jars requires Maintainer,
    // checked against `sender_id`, not the predecessor (the token contract).
    let msg = json!({ "type": "migrate", "data": [] });
    let result = ft::ft_transfer_call(&context.ft, nobody, context.jar.id(), 1, msg.to_string()).await?;
    assert!(result.has_panic("Only accounts with the Maintainer role can migrate jars"));

    Ok(())
}

// StagingManager/UpgradeManager (near-plugins' Upgradable: up_stage_code /
// up_deploy_code) are covered in upgrade.rs, alongside the real redeploy test —
// same structure dev-v2 uses.

/// Granting a role via `acl_grant_role` (the standard near-plugins method,
/// exercised here directly rather than through `all_roles_to`) unblocks the
/// gated method it protects, and does *not* unblock a different role's gate.
#[tokio::test]
async fn granting_a_role_unblocks_only_that_gate() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;
    let oracle_only = &context.bob;

    context
        .manager
        .call(context.jar.id(), "acl_grant_role")
        .args_json(json!({ "role": "Oracle", "account_id": oracle_only.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Granted role: gets past the ACL check (whatever panic follows is a
    // different, business-logic one — not an ACL rejection).
    let result = jar::record_score(
        &context.jar,
        oracle_only,
        vec![(context.alice.id(), vec![(100, 0.into())])],
    )
    .await?;
    assert!(!result.has_panic("Insufficient permissions"));

    // Same account, a gate for a role it was never granted.
    let result = jar::set_enabled(
        &context.jar,
        oracle_only,
        &RegisterProductCommand::Locked12Months12Percents.id(),
        false,
    )
    .await;
    assert!(result.is_err());

    Ok(())
}
