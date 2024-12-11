use nitka::misc::ToNear;
use sweat_jar_model::api::{FeeApiIntegration, WithdrawApiIntegration};
use sweat_model::FungibleTokenCoreIntegration;

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn test_fixed_withdraw_fee() -> anyhow::Result<()> {
    println!("👷🏽 Run fixed withdraw fee test");

    let mut context = prepare_contract(
        None,
        [RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee],
    )
    .await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;
    let fee_account = context.fee().await?;

    let fee_balance_before = context.ft_contract().ft_balance_of(fee_account.to_near()).await?.0;

    let product_id = RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee.id();
    context
        .sweat_jar()
        .create_jar(&alice, product_id.clone(), 1_000_000, &context.ft_contract())
        .await?;

    let mut alice_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?;
    assert_eq!(99_999_999_999_999_999_999_000_000, alice_balance.0);

    context.fast_forward_hours(1).await?;

    let withdraw_result = context.sweat_jar().withdraw(product_id).with_user(&alice).await?;
    let withdrawn_amount = withdraw_result.withdrawn_amount;
    let fee_amount = withdraw_result.fee;

    assert_eq!(999_000, withdrawn_amount.0);
    assert_eq!(1_000, fee_amount.0);

    alice_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?;
    assert_eq!(99_999_999_999_999_999_999_999_000, alice_balance.0);

    let expected_fee = 1_000;
    let available_fee = context.sweat_jar().get_fee_amount().await?.0;
    assert_eq!(expected_fee, available_fee);

    let withdrawn_fee = context.sweat_jar().withdraw_fee().with_user(&manager).await?.0;
    assert_eq!(expected_fee, withdrawn_fee);

    let fee_balance_after = context.ft_contract().ft_balance_of(fee_account.to_near()).await?.0;
    assert_eq!(expected_fee, fee_balance_after - fee_balance_before);

    Ok(())
}

#[tokio::test]
async fn test_percent_withdraw_fee() -> anyhow::Result<()> {
    println!("👷🏽 Run percent withdraw fee test");

    let mut context = prepare_contract(
        None,
        [RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee],
    )
    .await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;
    let fee_account = context.fee().await?;

    let fee_balance_before = context.ft_contract().ft_balance_of(fee_account.to_near()).await?.0;

    let product_id = RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee.id();
    context
        .sweat_jar()
        .create_jar(&alice, product_id.clone(), 1_000_000, &context.ft_contract())
        .await?;

    let mut alice_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?;
    assert_eq!(99_999_999_999_999_999_999_000_000, alice_balance.0);

    context.fast_forward_hours(1).await?;

    let withdraw_result = context.sweat_jar().withdraw(product_id).with_user(&alice).await?;
    let withdrawn_amount = withdraw_result.withdrawn_amount;
    let fee_amount = withdraw_result.fee;

    assert_eq!(990_000, withdrawn_amount.0);
    assert_eq!(10_000, fee_amount.0);

    alice_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?;
    assert_eq!(99_999_999_999_999_999_999_990_000, alice_balance.0);

    let expected_fee = 10_000;
    let available_fee = context.sweat_jar().get_fee_amount().await?.0;
    assert_eq!(expected_fee, available_fee);

    let withdrawn_fee = context.sweat_jar().withdraw_fee().with_user(&manager).await?.0;
    assert_eq!(expected_fee, withdrawn_fee);

    let fee_balance_after = context.ft_contract().ft_balance_of(fee_account.to_near()).await?.0;
    assert_eq!(expected_fee, fee_balance_after - fee_balance_before);

    Ok(())
}
