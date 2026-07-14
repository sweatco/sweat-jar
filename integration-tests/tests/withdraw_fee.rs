mod common;

use anyhow::Result;
use common::{ft, jar, prepare::prepare_contract, product::RegisterProductCommand};
use sweat_jar_model::U32;

#[tokio::test]
async fn test_fixed_withdraw_fee() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee]).await?;
    let alice = &context.alice;

    let fee_balance_before = ft::ft_balance_of(&context.ft, context.fee.id()).await?;

    jar::create_jar(
        &context.ft,
        &context.jar,
        alice,
        &RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee.id(),
        1_000_000,
    )
    .await?
    .into_result()?;

    let mut alice_balance = ft::ft_balance_of(&context.ft, alice.id()).await?;
    assert_eq!(99_000_000, alice_balance);

    // Product lockup is 10 minutes; forward just past maturity.
    context.fast_forward_minutes(11).await?;

    let withdraw_result = jar::withdraw(&context.jar, alice, U32(1), None).await?;

    assert_eq!(999_000, withdraw_result.withdrawn_amount.0);
    assert_eq!(1_000, withdraw_result.fee.0);

    alice_balance = ft::ft_balance_of(&context.ft, alice.id()).await?;
    assert_eq!(99_999_000, alice_balance);

    let fee_balance_after = ft::ft_balance_of(&context.ft, context.fee.id()).await?;
    assert_eq!(1_000, fee_balance_after - fee_balance_before);

    Ok(())
}

#[tokio::test]
async fn test_percent_withdraw_fee() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee]).await?;
    let alice = &context.alice;

    let fee_balance_before = ft::ft_balance_of(&context.ft, context.fee.id()).await?;

    jar::create_jar(
        &context.ft,
        &context.jar,
        alice,
        &RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee.id(),
        1_000_000,
    )
    .await?
    .into_result()?;

    let mut alice_balance = ft::ft_balance_of(&context.ft, alice.id()).await?;
    assert_eq!(99_000_000, alice_balance);

    // Product lockup is 10 minutes; forward just past maturity.
    context.fast_forward_minutes(11).await?;

    let withdraw_result = jar::withdraw(&context.jar, alice, U32(1), None).await?;

    assert_eq!(990_000, withdraw_result.withdrawn_amount.0);
    assert_eq!(10_000, withdraw_result.fee.0);

    alice_balance = ft::ft_balance_of(&context.ft, alice.id()).await?;
    assert_eq!(99_990_000, alice_balance);

    let fee_balance_after = ft::ft_balance_of(&context.ft, context.fee.id()).await?;
    assert_eq!(10_000, fee_balance_after - fee_balance_before);

    Ok(())
}
