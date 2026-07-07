use std::collections::HashSet;

use anyhow::Result;
use near_sdk::json_types::Base64VecU8;
use near_workspaces::{
    result::{ExecutionFinalResult, ExecutionSuccess},
    types::NearToken,
    Account, AccountId, Contract,
};
use serde_json::{json, Value};
use sweat_jar_model::data::{
    claim::ClaimedAmountView,
    deposit::DepositTicket,
    jar::{AggregatedInterestView, JarsView},
    product::{Product, ProductId},
    withdraw::{BulkWithdrawView, WithdrawView},
};

type U128 = u128;

pub async fn init(
    jar: &Contract,
    token_account_id: &AccountId,
    fee_account_id: &AccountId,
    previous_version_account_id: &AccountId,
    super_admin: &AccountId,
    roles: &RoleAssignments,
) -> Result<()> {
    jar.call("init")
        .args_json(json!({
            "token_account_id": token_account_id,
            "fee_account_id": fee_account_id,
            "previous_version_account_id": previous_version_account_id,
            "super_admin": super_admin,
            "roles": roles,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

/// JSON shape matching the contract's `RoleAssignments` — one list of account
/// IDs per `AccessControllable` role. Used as the `init`/migration argument.
#[derive(serde::Serialize)]
pub struct RoleAssignments {
    pub oracle: Vec<AccountId>,
    pub product_manager: Vec<AccountId>,
    pub fee_manager: Vec<AccountId>,
    pub maintainer: Vec<AccountId>,
    pub staging_manager: Vec<AccountId>,
    pub upgrade_manager: Vec<AccountId>,
}

/// Convenience for the common case: one account holding every role, matching
/// the previous `grant_all_roles` behavior but granted atomically at `init`
/// time instead of via a separate post-init call.
pub fn all_roles_to(account_id: &AccountId) -> RoleAssignments {
    RoleAssignments {
        oracle: vec![account_id.clone()],
        product_manager: vec![account_id.clone()],
        fee_manager: vec![account_id.clone()],
        maintainer: vec![account_id.clone()],
        staging_manager: vec![account_id.clone()],
        upgrade_manager: vec![account_id.clone()],
    }
}

/// Grants all 4 original `AccessControllable` roles to `account_id`, self-signed
/// by the jar contract account. Only succeeds if the jar contract itself
/// currently holds admin power for those roles — since `init`'s `super_admin`
/// argument is explicit and typically names a different account (see
/// `jar::init`/`all_roles_to`), the jar contract usually does NOT retain admin
/// power after init, so this self-signed call will silently no-op in that case
/// (`acl_grant_role` returns `None` rather than panicking on insufficient
/// permission). Prefer granting roles at `init` time via `RoleAssignments`, or
/// have the actual super-admin account sign the `acl_grant_role` call directly.
pub async fn grant_all_roles(jar: &Contract, account_id: &AccountId) -> Result<()> {
    for role in ["Oracle", "ProductManager", "FeeManager", "Maintainer"] {
        jar.call("acl_grant_role")
            .args_json(json!({ "role": role, "account_id": account_id }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
    }
    Ok(())
}

/// Grants a single `AccessControllable` role to `account_id`, self-signed by
/// the jar contract account. Unlike `grant_all_roles`, this lets a test hold
/// exactly one role and verify the other is still denied — but is subject to
/// the same caveat: it only succeeds if the jar contract itself currently
/// holds admin power for that role (see `grant_all_roles`'s doc comment).
pub async fn grant_role(jar: &Contract, role: &str, account_id: &AccountId) -> Result<()> {
    jar.call("acl_grant_role")
        .args_json(json!({ "role": role, "account_id": account_id }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn get_products(jar: &Contract) -> Result<Vec<Product>> {
    Ok(jar.view("get_products").await?.json()?)
}

pub async fn get_jars_for_account(jar: &Contract, account_id: &AccountId) -> Result<JarsView> {
    Ok(jar
        .view("get_jars_for_account")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?)
}

pub async fn get_total_interest(jar: &Contract, account_id: &AccountId) -> Result<AggregatedInterestView> {
    Ok(jar
        .view("get_total_interest")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?)
}

pub async fn get_fee_amount(jar: &Contract) -> Result<U128> {
    let value: Value = jar.view("get_fee_amount").await?.json()?;
    Ok(value.as_str().unwrap().parse()?)
}

pub async fn get_score(jar: &Contract, account_id: &AccountId) -> Result<Option<U128>> {
    let value: Option<Value> = jar
        .view("get_score")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?;
    Ok(value.map(|v| v.as_str().unwrap().parse().unwrap()))
}

pub async fn is_penalty_applied(jar: &Contract, account_id: &AccountId) -> Result<bool> {
    Ok(jar
        .view("is_penalty_applied")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?)
}

pub async fn block_timestamp_ms(jar: &Contract) -> Result<u64> {
    Ok(jar.view("block_timestamp_ms").await?.json()?)
}

pub async fn register_product(jar: &Contract, manager: &Account, product: Product) -> Result<()> {
    manager
        .call(jar.id(), "register_product")
        .args_json(json!({ "product": product }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn set_enabled(jar: &Contract, manager: &Account, product_id: &str, is_enabled: bool) -> Result<()> {
    manager
        .call(jar.id(), "set_enabled")
        .args_json(json!({ "product_id": product_id, "is_enabled": is_enabled }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn set_public_key(
    jar: &Contract,
    manager: &Account,
    product_id: &str,
    public_key: Base64VecU8,
) -> Result<()> {
    manager
        .call(jar.id(), "set_public_key")
        .args_json(json!({ "product_id": product_id, "public_key": public_key }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn claim_total(jar: &Contract, user: &Account, detailed: Option<bool>) -> Result<ClaimedAmountView> {
    Ok(user
        .call(jar.id(), "claim_total")
        .args_json(json!({ "detailed": detailed }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?)
}

pub async fn restake(
    jar: &Contract,
    user: &Account,
    from: &str,
    ticket: DepositTicket,
    signature: Option<Base64VecU8>,
    amount: Option<U128>,
) -> Result<()> {
    user.call(jar.id(), "restake")
        .args_json(json!({
            "from": from,
            "ticket": ticket,
            "signature": signature,
            "amount": amount.map(|a| a.to_string()),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn restake_all(
    jar: &Contract,
    user: &Account,
    ticket: DepositTicket,
    signature: Option<Base64VecU8>,
    amount: Option<U128>,
) -> Result<()> {
    user.call(jar.id(), "restake_all")
        .args_json(json!({
            "ticket": ticket,
            "signature": signature,
            "amount": amount.map(|a| a.to_string()),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn withdraw(jar: &Contract, user: &Account, product_id: &str) -> Result<WithdrawView> {
    Ok(user
        .call(jar.id(), "withdraw")
        .args_json(json!({ "product_id": product_id }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?)
}

pub async fn withdraw_all(
    jar: &Contract,
    user: &Account,
    product_ids: Option<HashSet<ProductId>>,
) -> Result<BulkWithdrawView> {
    Ok(user
        .call(jar.id(), "withdraw_all")
        .args_json(json!({ "product_ids": product_ids }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?)
}

pub async fn withdraw_fee(jar: &Contract, manager: &Account) -> Result<U128> {
    let value: Value = manager
        .call(jar.id(), "withdraw_fee")
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?;
    Ok(value.as_str().unwrap().parse()?)
}

pub async fn set_penalty(jar: &Contract, caller: &Account, account_id: &AccountId, value: bool) -> Result<()> {
    caller
        .call(jar.id(), "set_penalty")
        .args_json(json!({ "account_id": account_id, "value": value }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn set_feature_enabled(
    jar: &Contract,
    manager: &Account,
    account_id: &AccountId,
    feature: &str,
    enabled: bool,
) -> Result<()> {
    manager
        .call(jar.id(), "set_feature_enabled")
        .args_json(json!({ "account_id": account_id, "feature": feature, "enabled": enabled }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn unlock_jars_for_account(jar: &Contract, caller: &Account, account_id: &AccountId) -> Result<()> {
    caller
        .call(jar.id(), "unlock_jars_for_account")
        .args_json(json!({ "account_id": account_id }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn batch_set_feature_enabled(
    jar: &Contract,
    caller: &Account,
    account_ids: Vec<AccountId>,
    feature: &str,
    enabled: bool,
) -> Result<()> {
    caller
        .call(jar.id(), "batch_set_feature_enabled")
        .args_json(json!({ "account_ids": account_ids, "feature": feature, "enabled": enabled }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn record_score(jar: &Contract, manager: &Account, batch: Vec<(AccountId, Vec<(u16, u64)>)>) -> Result<()> {
    manager
        .call(jar.id(), "record_score")
        .args_json(json!({ "batch": batch }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn apply_booster(
    jar: &Contract,
    manager: &Account,
    account_ids: Vec<AccountId>,
    score: u16,
    timestamp: u64,
) -> Result<ExecutionSuccess> {
    Ok(manager
        .call(jar.id(), "apply_booster")
        .args_json(json!({ "account_ids": account_ids, "score": score, "timestamp": timestamp }))
        .max_gas()
        .transact()
        .await?
        .into_result()?)
}

pub async fn set_time_scale(jar: &Contract, manager: &Account, time_scale: f64) -> Result<()> {
    manager
        .call(jar.id(), "set_time_scale")
        .args_json(json!({ "time_scale": time_scale }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn set_timezone(jar: &Contract, manager: &Account, account_id: &AccountId, timezone: i64) -> Result<()> {
    // `timezone: I64` on the contract side is near-sdk's string-wrapped i64.
    manager
        .call(jar.id(), "set_timezone")
        .args_json(json!({ "account_id": account_id, "timezone": timezone.to_string() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn bulk_create_jars(
    jar: &Contract,
    manager: &Account,
    account_id: &AccountId,
    product_id: &str,
    principal: u128,
    number_of_jars: u16,
) -> Result<()> {
    manager
        .call(jar.id(), "bulk_create_jars")
        .args_json(json!({
            "account_id": account_id,
            "product_id": product_id,
            "principal": principal,
            "number_of_jars": number_of_jars,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn seed_accounts(
    jar: &Contract,
    manager: &Account,
    product_id: &str,
    accounts: Vec<(AccountId, U128, i64)>,
    deposit_timestamp_ms: u64,
) -> Result<()> {
    manager
        .call(jar.id(), "seed_accounts")
        .args_json(json!({
            "product_id": product_id,
            "accounts": accounts.into_iter().map(|(id, principal, tz)| (id, principal.to_string(), tz)).collect::<Vec<_>>(),
            "deposit_timestamp_ms": deposit_timestamp_ms,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

/// Raw variant: returns the full `ExecutionFinalResult` instead of unwrapping
/// to `U128`. Needed when a caller expects the create to be rejected, since a
/// rejected `ft_on_transfer` doesn't surface as an `Err` from the outer
/// `ft_transfer_call` (NEP-141's resolve_transfer refunds the full amount and
/// the outer call still resolves to `Ok("0")`) — the panic message only shows
/// up in the nested receipt captured in the raw result's `Debug` output.
async fn create_jar_with_msg_raw(
    jar: &Contract,
    ft: &Contract,
    user: &Account,
    msg: Value,
    amount: u128,
) -> Result<ExecutionFinalResult> {
    tracing::info!("creating jar with msg: {msg:?}");
    Ok(user
        .call(ft.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": jar.id(),
            "amount": amount.to_string(),
            "memo": Option::<String>::None,
            "msg": msg.to_string(),
        }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?)
}

async fn create_jar_with_msg(jar: &Contract, ft: &Contract, user: &Account, msg: Value, amount: u128) -> Result<U128> {
    let value: Value = create_jar_with_msg_raw(jar, ft, user, msg, amount)
        .await?
        .into_result()?
        .json()?;
    Ok(value.as_str().unwrap().parse()?)
}

pub async fn create_jar(jar: &Contract, ft: &Contract, user: &Account, product_id: &str, amount: u128) -> Result<U128> {
    let msg = json!({
        "type": "stake",
        "data": { "ticket": { "product_id": product_id, "valid_until": "0" } }
    });
    create_jar_with_msg(jar, ft, user, msg, amount).await
}

/// Raw variant of `create_jar` for tests that expect the on-chain create to be
/// rejected (see `create_jar_with_msg_raw`'s doc comment for why the plain
/// `Result<U128>` path can't observe that rejection).
pub async fn create_jar_raw(
    jar: &Contract,
    ft: &Contract,
    user: &Account,
    product_id: &str,
    amount: u128,
) -> Result<ExecutionFinalResult> {
    let msg = json!({
        "type": "stake",
        "data": { "ticket": { "product_id": product_id, "valid_until": "0" } }
    });
    create_jar_with_msg_raw(jar, ft, user, msg, amount).await
}

pub async fn create_step_jar(
    jar: &Contract,
    ft: &Contract,
    user: &Account,
    product_id: &str,
    amount: u128,
    signature: Base64VecU8,
    valid_until: u64,
    timezone: i64,
) -> Result<U128> {
    let msg = json!({
        "type": "stake",
        "data": {
            "ticket": { "product_id": product_id, "valid_until": valid_until.to_string(), "timezone": timezone },
            "signature": signature,
        }
    });
    create_jar_with_msg(jar, ft, user, msg, amount).await
}

pub async fn create_premium_jar(
    jar: &Contract,
    ft: &Contract,
    user: &Account,
    product_id: &str,
    amount: u128,
    signature: Base64VecU8,
    valid_until: u64,
) -> Result<U128> {
    let msg = json!({
        "type": "stake",
        "data": {
            "ticket": { "product_id": product_id, "valid_until": valid_until.to_string() },
            "signature": signature,
        }
    });
    create_jar_with_msg(jar, ft, user, msg, amount).await
}

pub async fn airdrop(
    jar: &Contract,
    ft: &Contract,
    manager: &Account,
    product_id: &str,
    amount_per_receiver: u128,
    receivers: &[Account],
    timezone: Option<i64>,
) -> Result<U128> {
    let receiver_ids: Vec<&AccountId> = receivers.iter().map(Account::id).collect();
    let total_amount = amount_per_receiver * receiver_ids.len() as u128;
    let msg = json!({
        "type": "airdrop",
        "data": {
            "ticket": { "product_id": product_id, "valid_until": "0", "timezone": timezone },
            "receivers": receiver_ids,
        }
    });
    create_jar_with_msg(jar, ft, manager, msg, total_amount).await
}

pub async fn airdrop_protected(
    jar: &Contract,
    ft: &Contract,
    manager: &Account,
    product_id: &str,
    amount_per_receiver: u128,
    receivers: &[Account],
    timezone: Option<i64>,
    signature: Base64VecU8,
    valid_until: u64,
    booster: u16,
    booster_timestamp: Option<u64>,
) -> Result<U128> {
    let value: Value = airdrop_protected_raw(
        jar,
        ft,
        manager,
        product_id,
        amount_per_receiver,
        receivers,
        timezone,
        signature,
        valid_until,
        booster,
        booster_timestamp,
    )
    .await?
    .into_result()?
    .json()?;
    Ok(value.as_str().unwrap().parse()?)
}

/// Raw variant of `airdrop_protected` for tests that need to inspect the
/// emitted `ApplyBooster` event directly — the plain `Result<U128>` path only
/// surfaces the deposit total, which doesn't change whether a booster was
/// actually applied or silently rejected (booster and principal are tracked
/// independently; `get_score`'s view only reflects recorded score, never the
/// booster field, so the event log is the only observable proof of booster
/// application).
#[allow(clippy::too_many_arguments)]
pub async fn airdrop_protected_raw(
    jar: &Contract,
    ft: &Contract,
    manager: &Account,
    product_id: &str,
    amount_per_receiver: u128,
    receivers: &[Account],
    timezone: Option<i64>,
    signature: Base64VecU8,
    valid_until: u64,
    booster: u16,
    booster_timestamp: Option<u64>,
) -> Result<ExecutionFinalResult> {
    let receiver_ids: Vec<&AccountId> = receivers.iter().map(Account::id).collect();
    let total_amount = amount_per_receiver * receiver_ids.len() as u128;
    let booster_opt = if booster > 0 { Some(booster) } else { None };
    let msg = json!({
        "type": "airdrop",
        "data": {
            "ticket": { "product_id": product_id, "valid_until": valid_until.to_string(), "timezone": timezone },
            "signature": signature,
            "receivers": receiver_ids,
            "booster": booster_opt,
            "booster_timestamp": booster_timestamp,
        }
    });
    create_jar_with_msg_raw(jar, ft, manager, msg, total_amount).await
}
