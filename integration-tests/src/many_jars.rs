use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::{
    api::{ClaimApiIntegration, IntegrationTestMethodsIntegration, JarApiIntegration, WithdrawApiIntegration},
    claimed_amount_view::ClaimedAmountView,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand::Locked5Minutes60000Percents,
};

#[tokio::test]
#[mutants::skip]
async fn claim_many_jars() -> Result<()> {
    const DEPOSIT_PRINCIPAL: u128 = 1_000;
    const DEPOSITS_COUNT: usize = 20_000;

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
        context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len(),
        DEPOSITS_COUNT
    );

    context.fast_forward_minutes(5).await?;

    let claimed = context.sweat_jar().claim_total(true.into()).with_user(&alice).await?;

    let batch_claim_summ = claimed.get_total().0;

    dbg!(&batch_claim_summ);

    assert_eq!(
        batch_claim_summ * 9,
        context
            .sweat_jar()
            .get_total_interest(alice.to_near())
            .await?
            .amount
            .total
            .0
    );

    for i in 1..10 {
        let claimed = context.sweat_jar().claim_total(true.into()).with_user(&alice).await?;
        assert_eq!(claimed.get_total().0, batch_claim_summ);

        assert_eq!(
            batch_claim_summ * (9 - i),
            context
                .sweat_jar()
                .get_total_interest(alice.to_near())
                .await?
                .amount
                .total
                .0
        );
    }

    assert_eq!(
        context
            .sweat_jar()
            .get_total_interest(alice.to_near())
            .await?
            .amount
            .total
            .0,
        0
    );

    assert_eq!(
        context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len(),
        2000
    );

    let withdrawn = context.sweat_jar().withdraw_all().with_user(&alice).await?;
    assert_eq!(1, withdrawn.withdrawals.len());
    assert_eq!(DEPOSITS_COUNT as u128 * DEPOSIT_PRINCIPAL, withdrawn.total_amount.0);

    assert_eq!(
        context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len(),
        0
    );

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn restake_many_jars() -> Result<()> {
    const DEPOSIT_PRINCIPAL: u128 = 50 * (10 ^ 18);
    const DEPOSITS_COUNT: usize = 15_000;

    println!("👷🏽 Restake many jars test");

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

    let original_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(original_jars.len(), DEPOSITS_COUNT);

    let mut original_dates: Vec<u64> = original_jars.iter().map(|jar| jar.created_at.0).collect();
    original_dates.sort();
    let original_date_latest = original_dates.last().unwrap().clone();

    context.fast_forward_minutes(5).await?;

    let ClaimedAmountView::Detailed(claimed) = context.sweat_jar().claim_total(true.into()).with_user(&alice).await?
    else {
        panic!();
    };
    assert_eq!(1, claimed.detailed.len());

    let restaked = context.sweat_jar().restake_all(None).with_user(&alice).await?;
    assert_eq!(1, restaked.len());
    assert_eq!(DEPOSITS_COUNT as u128 * DEPOSIT_PRINCIPAL, restaked.first().unwrap().1);

    let restaked_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(1, restaked_jars.len());
    let restake_date = restaked_jars.first().unwrap().created_at.0;

    assert!(original_date_latest < restake_date);

    Ok(())
}
