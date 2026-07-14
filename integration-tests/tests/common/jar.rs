use anyhow::Result;
use near_workspaces::{result::ExecutionFinalResult, types::NearToken, Account, AccountId, Contract};
use serde_json::{json, Value};
pub use sweat_jar::{all_roles_to, RoleAssignments};
use sweat_jar_model::{
    claimed_amount_view::ClaimedAmountView,
    jar::{AggregatedInterestView, AggregatedTokenAmountView, JarIdView, JarView},
    product::ProductView,
    withdraw::{BulkWithdrawView, WithdrawView},
    Score, Timezone, UTC,
};

use super::ft;

pub async fn init(
    jar: &Contract,
    token_account_id: &AccountId,
    fee_account_id: &AccountId,
    new_version_account_id: &AccountId,
    super_admin: &AccountId,
    roles: &RoleAssignments,
) -> Result<()> {
    jar.call("init")
        .args_json(json!({
            "token_account_id": token_account_id,
            "fee_account_id": fee_account_id,
            "new_version_account_id": new_version_account_id,
            "super_admin": super_admin,
            "roles": roles,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn register_product(jar: &Contract, manager: &Account, command: Value) -> Result<()> {
    manager
        .call(jar.id(), "register_product")
        .args_json(json!({ "command": command }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn get_products(jar: &Contract) -> Result<Vec<ProductView>> {
    Ok(jar.view("get_products").await?.json()?)
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
    public_key_base64: &str,
) -> Result<()> {
    manager
        .call(jar.id(), "set_public_key")
        .args_json(json!({ "product_id": product_id, "public_key": public_key_base64 }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

/// Stakes `amount` by transferring it from `user` to the jar contract with a
/// `stake` message. Returns the raw outcome; use [`used_amount`] to extract
/// how much the jar accepted.
pub async fn create_jar(
    context_ft: &Contract,
    jar: &Contract,
    user: &Account,
    product_id: &str,
    amount: u128,
) -> Result<ExecutionFinalResult> {
    let msg = json!({
        "type": "stake",
        "data": {
            "ticket": {
                "product_id": product_id,
                "valid_until": "0",
            }
        }
    });

    ft::ft_transfer_call(context_ft, user, jar.id(), amount, msg.to_string()).await
}

pub async fn create_step_jar(
    context_ft: &Contract,
    jar: &Contract,
    user: &Account,
    product_id: &str,
    amount: u128,
    timezone: Timezone,
) -> Result<ExecutionFinalResult> {
    let msg = json!({
        "type": "stake",
        "data": {
            "ticket": {
                "product_id": product_id,
                "valid_until": "0",
                "timezone": timezone,
            }
        }
    });

    ft::ft_transfer_call(context_ft, user, jar.id(), amount, msg.to_string()).await
}

pub async fn create_premium_jar(
    context_ft: &Contract,
    jar: &Contract,
    user: &Account,
    product_id: &str,
    amount: u128,
    signature: &str,
    valid_until: u64,
) -> Result<ExecutionFinalResult> {
    let msg = json!({
        "type": "stake",
        "data": {
            "ticket": {
                "product_id": product_id,
                "valid_until": valid_until.to_string(),
            },
            "signature": signature,
        }
    });

    ft::ft_transfer_call(context_ft, user, jar.id(), amount, msg.to_string()).await
}

pub async fn top_up(
    context_ft: &Contract,
    jar: &Contract,
    user: &Account,
    jar_id: JarIdView,
    amount: u128,
) -> Result<ExecutionFinalResult> {
    let msg = json!({
        "type": "top_up",
        "data": jar_id,
    });

    ft::ft_transfer_call(context_ft, user, jar.id(), amount, msg.to_string()).await
}

/// The amount the jar accepted from an `ft_transfer_call` staking outcome.
pub fn used_amount(result: ExecutionFinalResult) -> Result<u128> {
    let used: String = result.into_result()?.json()?;
    Ok(used.parse()?)
}

/// Message the oracle signs (hashed with SHA-256) to authorize a premium deposit.
pub fn signature_material(
    jar: &Contract,
    receiver_id: &AccountId,
    product_id: &str,
    valid_until: u64,
    amount: u128,
    last_jar_id: Option<String>,
) -> String {
    format!(
        "{},{},{},{},{},{}",
        jar.id(),
        receiver_id,
        product_id,
        amount,
        last_jar_id.unwrap_or_default(),
        valid_until,
    )
}

pub async fn get_jar(jar: &Contract, account_id: &AccountId, jar_id: JarIdView) -> Result<JarView> {
    Ok(jar
        .view("get_jar")
        .args_json(json!({ "account_id": account_id, "jar_id": jar_id }))
        .await?
        .json()?)
}

pub async fn get_jars_for_account(jar: &Contract, account_id: &AccountId) -> Result<Vec<JarView>> {
    Ok(jar
        .view("get_jars_for_account")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?)
}

pub async fn get_total_principal(jar: &Contract, account_id: &AccountId) -> Result<AggregatedTokenAmountView> {
    Ok(jar
        .view("get_total_principal")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?)
}

pub async fn get_principal(
    jar: &Contract,
    account_id: &AccountId,
    jar_ids: Vec<JarIdView>,
) -> Result<AggregatedTokenAmountView> {
    Ok(jar
        .view("get_principal")
        .args_json(json!({ "account_id": account_id, "jar_ids": jar_ids }))
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

pub async fn get_interest(
    jar: &Contract,
    account_id: &AccountId,
    jar_ids: Vec<JarIdView>,
) -> Result<AggregatedInterestView> {
    Ok(jar
        .view("get_interest")
        .args_json(json!({ "account_id": account_id, "jar_ids": jar_ids }))
        .await?
        .json()?)
}

/// Raw `claim_total` outcome — for panic inspection and gas measurement.
pub async fn claim_total_raw(jar: &Contract, user: &Account, detailed: Option<bool>) -> Result<ExecutionFinalResult> {
    Ok(user
        .call(jar.id(), "claim_total")
        .args_json(json!({ "detailed": detailed }))
        .max_gas()
        .transact()
        .await?)
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

pub async fn withdraw(jar: &Contract, user: &Account, jar_id: JarIdView, amount: Option<u128>) -> Result<WithdrawView> {
    Ok(user
        .call(jar.id(), "withdraw")
        .args_json(json!({ "jar_id": jar_id, "amount": amount.map(|a| a.to_string()) }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?)
}

pub async fn withdraw_all(jar: &Contract, user: &Account) -> Result<BulkWithdrawView> {
    Ok(user
        .call(jar.id(), "withdraw_all")
        .args_json(json!({ "jars": Option::<Vec<JarIdView>>::None }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?)
}

pub async fn restake(jar: &Contract, user: &Account, jar_id: JarIdView) -> Result<JarView> {
    Ok(user
        .call(jar.id(), "restake")
        .args_json(json!({ "jar_id": jar_id }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?)
}

pub async fn restake_all(jar: &Contract, user: &Account) -> Result<Vec<JarView>> {
    Ok(user
        .call(jar.id(), "restake_all")
        .args_json(json!({ "jars": Option::<Vec<JarIdView>>::None }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json()?)
}

pub async fn set_penalty(
    jar: &Contract,
    caller: &Account,
    account_id: &AccountId,
    jar_id: JarIdView,
    value: bool,
) -> Result<ExecutionFinalResult> {
    Ok(caller
        .call(jar.id(), "set_penalty")
        .args_json(json!({ "account_id": account_id, "jar_id": jar_id, "value": value }))
        .max_gas()
        .transact()
        .await?)
}

pub async fn batch_set_penalty(
    jar: &Contract,
    caller: &Account,
    jars: Vec<(&AccountId, Vec<JarIdView>)>,
    value: bool,
) -> Result<ExecutionFinalResult> {
    Ok(caller
        .call(jar.id(), "batch_set_penalty")
        .args_json(json!({ "jars": jars, "value": value }))
        .max_gas()
        .transact()
        .await?)
}

pub async fn unlock_jars_for_account(
    jar: &Contract,
    caller: &Account,
    account_id: &AccountId,
) -> Result<ExecutionFinalResult> {
    Ok(caller
        .call(jar.id(), "unlock_jars_for_account")
        .args_json(json!({ "account_id": account_id }))
        .max_gas()
        .transact()
        .await?)
}

pub async fn update_contract(jar: &Contract, caller: &Account) -> Result<ExecutionFinalResult> {
    Ok(caller
        .call(jar.id(), "update_contract")
        .args_json(json!({ "code": Vec::<u8>::new(), "callback": Option::<String>::None }))
        .max_gas()
        .transact()
        .await?)
}

pub async fn record_score(
    jar: &Contract,
    manager: &Account,
    batch: Vec<(&AccountId, Vec<(Score, UTC)>)>,
) -> Result<ExecutionFinalResult> {
    Ok(manager
        .call(jar.id(), "record_score")
        .args_json(json!({ "batch": batch }))
        .max_gas()
        .transact()
        .await?)
}

pub async fn block_timestamp_ms(jar: &Contract) -> Result<u64> {
    Ok(jar.view("block_timestamp_ms").await?.json()?)
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

pub async fn migrate_products(jar: &Contract, manager: &Account) -> Result<ExecutionFinalResult> {
    Ok(manager.call(jar.id(), "migrate_products").max_gas().transact().await?)
}

pub async fn migrate_account(jar: &Contract, user: &Account) -> Result<ExecutionFinalResult> {
    Ok(user.call(jar.id(), "migrate_account").max_gas().transact().await?)
}

pub async fn force_migrate_account(
    jar: &Contract,
    caller: &Account,
    account_id: &AccountId,
) -> Result<ExecutionFinalResult> {
    Ok(caller
        .call(jar.id(), "force_migrate_account")
        .args_json(json!({ "account_id": account_id }))
        .max_gas()
        .transact()
        .await?)
}

pub async fn unlock_account(jar: &Contract, caller: &Account, account_id: &AccountId) -> Result<ExecutionFinalResult> {
    Ok(caller
        .call(jar.id(), "unlock_account")
        .args_json(json!({ "account_id": account_id }))
        .max_gas()
        .transact()
        .await?)
}
