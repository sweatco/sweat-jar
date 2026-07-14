use anyhow::Result;
use near_workspaces::{result::ExecutionFinalResult, types::NearToken, Account, AccountId, Contract};
use serde_json::{json, Value};

pub async fn new(ft: &Contract, postfix: &str) -> Result<()> {
    ft.call("new")
        .args_json(json!({ "postfix": postfix }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn storage_deposit(ft: &Contract, account_id: &AccountId) -> Result<()> {
    let bounds: Value = ft.view("storage_balance_bounds").await?.json()?;
    let min: u128 = bounds.get("min").and_then(Value::as_str).unwrap().parse()?;
    ft.call("storage_deposit")
        .args_json(json!({ "account_id": account_id }))
        .deposit(NearToken::from_yoctonear(min))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn tge_mint(ft: &Contract, account_id: &AccountId, amount: u128) -> Result<()> {
    ft.call("tge_mint")
        .args_json(json!({ "account_id": account_id, "amount": amount.to_string() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

pub async fn ft_transfer(ft: &Contract, from: &Account, to: &AccountId, amount: u128) -> Result<()> {
    from.call(ft.id(), "ft_transfer")
        .args_json(json!({ "receiver_id": to, "amount": amount.to_string(), "memo": Option::<String>::None }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

/// `ft_transfer_call` from `from` to the jar contract; `msg` drives the jar's
/// `ft_on_transfer` (stake / top-up). Returns the raw outcome so callers can
/// inspect panics or the used amount.
pub async fn ft_transfer_call(
    ft: &Contract,
    from: &Account,
    receiver_id: &AccountId,
    amount: u128,
    msg: String,
) -> Result<ExecutionFinalResult> {
    Ok(from
        .call(ft.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": receiver_id,
            "amount": amount.to_string(),
            "memo": Option::<String>::None,
            "msg": msg,
        }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?)
}

pub async fn ft_balance_of(ft: &Contract, account_id: &AccountId) -> Result<u128> {
    let balance: String = ft
        .view("ft_balance_of")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?;
    Ok(balance.parse()?)
}
