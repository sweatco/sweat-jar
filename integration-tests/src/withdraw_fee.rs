#![cfg(test)]

use workspaces::Account;

use crate::{common::ValueGetters, context::Context, product::RegisterProductCommand};

#[tokio::test]
pub async fn withdraw_fee() -> anyhow::Result<()> {
    println!("👷🏽 Run withdraw fee test");

    test_fixed_fee().await?;
    test_percent_fee().await?;

    Ok(())
}

async fn test_fixed_fee() -> anyhow::Result<()> {
    let (context, alice, manager, fee_account) = prepare_contract().await?;

    let fee_balance_before = context.ft_contract.ft_balance_of(&fee_account).await?.0;

    context
        .jar_contract
        .register_product(
            &manager,
            RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee.json(),
        )
        .await?;

    context
        .jar_contract
        .create_jar(
            &alice,
            RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee.id(),
            1_000_000,
            context.ft_contract.account().id(),
        )
        .await?;

    let mut alice_balance = context.ft_contract.ft_balance_of(&alice).await?;
    assert_eq!(99_000_000, alice_balance.0);

    context.fast_forward(1).await?;

    let withdraw_result = context.jar_contract.withdraw(&alice, "0").await?;
    let withdrawn_amount = withdraw_result.get_u128("withdrawn_amount");
    let fee_amount = withdraw_result.get_u128("fee");

    assert_eq!(999_000, withdrawn_amount);
    assert_eq!(1_000, fee_amount);

    alice_balance = context.ft_contract.ft_balance_of(&alice).await?;
    assert_eq!(99_999_000, alice_balance.0);

    let fee_balance_after = context.ft_contract.ft_balance_of(&fee_account).await?.0;
    assert_eq!(1_000, fee_balance_after - fee_balance_before);

    Ok(())
}

async fn test_percent_fee() -> anyhow::Result<()> {
    let (context, alice, manager, fee_account) = prepare_contract().await?;

    let fee_balance_before = context.ft_contract.ft_balance_of(&fee_account).await?.0;

    context
        .jar_contract
        .register_product(
            &manager,
            RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee.json(),
        )
        .await?;

    context
        .jar_contract
        .create_jar(
            &alice,
            RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee.id(),
            1_000_000,
            context.ft_contract.account().id(),
        )
        .await?;

    let mut alice_balance = context.ft_contract.ft_balance_of(&alice).await?;
    assert_eq!(99_000_000, alice_balance.0);

    context.fast_forward(1).await?;

    let withdraw_result = context.jar_contract.withdraw(&alice, "0").await?;
    let withdrawn_amount = withdraw_result.get_u128("withdrawn_amount");
    let fee_amount = withdraw_result.get_u128("fee");

    assert_eq!(990_000, withdrawn_amount);
    assert_eq!(10_000, fee_amount);

    alice_balance = context.ft_contract.ft_balance_of(&alice).await?;
    assert_eq!(99_990_000, alice_balance.0);

    let fee_balance_after = context.ft_contract.ft_balance_of(&fee_account).await?.0;
    assert_eq!(10_000, fee_balance_after - fee_balance_before);

    Ok(())
}

async fn prepare_contract() -> anyhow::Result<(Context, Account, Account, Account)> {
    let mut context = Context::new().await?;

    let manager = &context.account("manager").await?;
    let alice = &context.account("alice").await?;
    let fee_account = &context.account("fee").await?;

    context.ft_contract.init().await?;
    context
        .jar_contract
        .init(context.ft_contract.account(), fee_account, manager.id())
        .await?;

    context
        .ft_contract
        .storage_deposit(context.jar_contract.account())
        .await?;
    context.ft_contract.storage_deposit(fee_account).await?;
    context.ft_contract.storage_deposit(alice).await?;
    context.ft_contract.mint_for_user(alice, 100_000_000).await?;

    Ok((context, alice.clone(), manager.clone(), fee_account.clone()))
}
