use std::collections::HashSet;

use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::api::*;
use sweat_model::FungibleTokenCoreIntegration;

use crate::{
    context::{prepare_contract, ContextHelpers, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn withdraw_all() -> Result<()> {
    const PRINCIPAL: u128 = 1_000_000;
    const JARS_COUNT: u16 = 500;
    const BULK_PRINCIPAL: u128 = PRINCIPAL * JARS_COUNT as u128;

    println!("👷🏽 Run test for withdraw all");

    set_integration_logs_enabled(false);

    let product_5_min = RegisterProductCommand::Locked5Minutes60000Percents;
    let product_10_min = RegisterProductCommand::Locked10Minutes60000Percents;

    let mut context = prepare_contract(None, [product_5_min, product_10_min]).await?;

    let alice = context.alice().await?;

    let mut product_5_min_total = 0;

    product_5_min_total += PRINCIPAL + 1;
    let amount = context
        .sweat_jar()
        .create_jar(&alice, product_5_min.id(), PRINCIPAL + 1, &context.ft_contract())
        .await?;
    assert_eq!(amount.0, PRINCIPAL + 1);

    let jar_5_min_1 = context.last_jar_for(&alice).await?;
    assert_eq!(jar_5_min_1.principal(), PRINCIPAL + 1);

    product_5_min_total += PRINCIPAL + 2;
    context
        .sweat_jar()
        .create_jar(&alice, product_5_min.id(), PRINCIPAL + 2, &context.ft_contract())
        .await?;
    let jar_5_min_2 = context.last_jar_for(&alice).await?;
    assert_eq!(jar_5_min_2.principal(), PRINCIPAL + 2);

    product_5_min_total += PRINCIPAL * JARS_COUNT as u128;
    context
        .bulk_create_jars(&alice, &product_5_min.id(), PRINCIPAL, JARS_COUNT)
        .await?;

    context
        .sweat_jar()
        .create_jar(&alice, product_10_min.id(), PRINCIPAL + 3, &context.ft_contract())
        .await?;
    let jar_10_min = context.last_jar_for(&alice).await?;
    assert_eq!(jar_10_min.principal(), PRINCIPAL + 3);

    let claimed = context.sweat_jar().claim_total(None).await;
    assert!(claimed.is_err());

    context.fast_forward_minutes(6).await?;

    // 2 calls to claim all 210 jars
    context.sweat_jar().claim_total(None).with_user(&alice).await?;
    context.sweat_jar().claim_total(None).with_user(&alice).await?;

    let alice_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?;
    let jar_balance = context
        .ft_contract()
        .ft_balance_of(context.sweat_jar().contract.as_account().to_near())
        .await?;

    let withdrawn = context.sweat_jar().withdraw_all(None).with_user(&alice).await?;
    assert_eq!(withdrawn.withdrawals.len(), 2);

    let alice_balance_after = context.ft_contract().ft_balance_of(alice.to_near()).await?;
    let jar_balance_after = context
        .ft_contract()
        .ft_balance_of(context.sweat_jar().contract.as_account().to_near())
        .await?;

    assert_eq!(alice_balance_after.0 - alice_balance.0, BULK_PRINCIPAL + 2000003);
    assert_eq!(jar_balance.0 - jar_balance_after.0, BULK_PRINCIPAL + 2000003);

    assert_eq!(withdrawn.total_amount.0, product_5_min_total);

    assert_eq!(
        withdrawn
            .withdrawals
            .iter()
            .map(|j| j.withdrawn_amount.0)
            .take(2)
            .collect::<HashSet<_>>(),
        vec![product_5_min_total, 0].into_iter().collect::<HashSet<_>>()
    );

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;

    assert_eq!(jars.0.get(&product_10_min.id()).unwrap().len(), 1);
    assert_eq!(
        jars.get_total_principal_for_product(&product_10_min.id()),
        jar_10_min.principal()
    );

    Ok(())
}
