use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::{
    api::*,
    data::{claim::ClaimedAmountView, deposit::DepositTicket},
};
use sweat_model::SweatApiIntegration;

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand::Locked5Minutes60000Percents,
};

#[tokio::test]
#[mutants::skip]
async fn claim_many_jars() -> Result<()> {
    const DEPOSIT_PRINCIPAL: u128 = 1_000;
    const DEPOSITS_COUNT: usize = 15_000;

    println!("👷🏽 Claim many jars test");

    set_integration_logs_enabled(false);

    let mut context = prepare_contract(None, [Locked5Minutes60000Percents]).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    context
        .sweat_jar()
        .bulk_create_jars(
            alice.to_near(),
            Locked5Minutes60000Percents.id(),
            DEPOSIT_PRINCIPAL,
            DEPOSITS_COUNT as u16,
        )
        .with_user(&manager)
        .await?;

    assert_eq!(
        context
            .sweat_jar()
            .get_jars_for_account(alice.to_near())
            .await?
            .get_total_deposits_number(),
        DEPOSITS_COUNT
    );

    context.fast_forward_minutes(10).await?;

    let claimed = context.sweat_jar().claim_total(true.into()).with_user(&alice).await?;
    let batch_claim_summ = claimed.get_total().0;
    assert_ne!(0, batch_claim_summ);

    assert_eq!(
        DEPOSITS_COUNT,
        context
            .sweat_jar()
            .get_jars_for_account(alice.to_near())
            .await?
            .get_total_deposits_number(),
    );

    let withdrawn = context.sweat_jar().withdraw_all(None).with_user(&alice).await?;
    assert_eq!(1, withdrawn.withdrawals.len());
    assert_eq!(DEPOSITS_COUNT as u128 * DEPOSIT_PRINCIPAL, withdrawn.total_amount.0);

    assert_eq!(
        context
            .sweat_jar()
            .get_jars_for_account(alice.to_near())
            .await?
            .get_total_deposits_number(),
        0
    );

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn restake_many_jars() -> Result<()> {
    const DEPOSIT_PRINCIPAL: u128 = 5 * 10u128.pow(18);
    const DEPOSITS_COUNT: usize = 15_000;

    println!("👷🏽 Restake many jars test");

    // set_integration_logs_enabled(false);

    let mut context = prepare_contract(None, [Locked5Minutes60000Percents]).await?;
    context
        .ft_contract()
        .tge_mint(context.sweat_jar().contract.id(), (100_000_000 * 10u128.pow(18)).into())
        .await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    let product_id = Locked5Minutes60000Percents.id();
    context
        .sweat_jar()
        .bulk_create_jars(
            alice.to_near(),
            product_id.clone(),
            DEPOSIT_PRINCIPAL,
            DEPOSITS_COUNT as u16,
        )
        .with_user(&manager)
        .await?;

    let original_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(original_jars.get_total_deposits_number(), DEPOSITS_COUNT);

    let mut original_dates: Vec<u64> = original_jars
        .0
        .values()
        .flat_map(|deposits| deposits.iter().map(|(timestamp, _)| timestamp))
        .cloned()
        .collect();
    original_dates.sort();
    let original_date_latest = original_dates.last().unwrap();

    context.fast_forward_minutes(10).await?;

    let ClaimedAmountView::Detailed(claimed) = context.sweat_jar().claim_total(true.into()).with_user(&alice).await?
    else {
        panic!();
    };
    assert_eq!(1, claimed.detailed.len());

    let ticket = DepositTicket {
        product_id: product_id.clone(),
        valid_until: 0.into(),
        timezone: None,
    };
    context
        .sweat_jar()
        .restake_all(ticket, None, None)
        .with_user(&alice)
        .result()
        .await?;

    let restaked_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(1, restaked_jars.get_total_deposits_number());
    let restake_date = restaked_jars.0.get(&product_id).unwrap().first().unwrap().0;

    assert!(*original_date_latest < restake_date);

    Ok(())
}
