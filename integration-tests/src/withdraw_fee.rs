use crate::{
    common::{prepare_contract, Prepared},
    product::RegisterProductCommand,
};

#[tokio::test]
async fn test_fixed_withdraw_fee() -> anyhow::Result<()> {
    println!("👷🏽 Run fixed withdraw fee test");

    let Prepared {
        context,
        manager: _,
        alice,
        fee_account,
    } = prepare_contract([RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee]).await?;

    let fee_balance_before = context.ft_contract.ft_balance_of(&fee_account).await?.0;

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

    context.fast_forward_hours(1).await?;

    let withdraw_result = context.jar_contract.withdraw(&alice, "1").await?;
    let withdrawn_amount = withdraw_result.withdrawn_amount;
    let fee_amount = withdraw_result.fee;

    assert_eq!(999_000, withdrawn_amount.0);
    assert_eq!(1_000, fee_amount.0);

    alice_balance = context.ft_contract.ft_balance_of(&alice).await?;
    assert_eq!(99_999_000, alice_balance.0);

    let fee_balance_after = context.ft_contract.ft_balance_of(&fee_account).await?.0;
    assert_eq!(1_000, fee_balance_after - fee_balance_before);

    Ok(())
}

#[tokio::test]
async fn test_percent_withdraw_fee() -> anyhow::Result<()> {
    println!("👷🏽 Run percent withdraw fee test");

    let Prepared {
        context,
        manager: _,
        alice,
        fee_account,
    } = prepare_contract([RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee]).await?;

    let fee_balance_before = context.ft_contract.ft_balance_of(&fee_account).await?.0;

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

    context.fast_forward_hours(1).await?;

    let withdraw_result = context.jar_contract.withdraw(&alice, "1").await?;
    let withdrawn_amount = withdraw_result.withdrawn_amount;
    let fee_amount = withdraw_result.fee;

    assert_eq!(990_000, withdrawn_amount.0);
    assert_eq!(10_000, fee_amount.0);

    alice_balance = context.ft_contract.ft_balance_of(&alice).await?;
    assert_eq!(99_990_000, alice_balance.0);

    let fee_balance_after = context.ft_contract.ft_balance_of(&fee_account).await?.0;
    assert_eq!(10_000, fee_balance_after - fee_balance_before);

    Ok(())
}
