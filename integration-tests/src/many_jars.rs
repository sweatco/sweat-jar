use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::{
    api::{ClaimApiIntegration, IntegrationTestMethodsIntegration, JarApiIntegration, WithdrawApiIntegration},
    claimed_amount_view::ClaimedAmountView,
    JAR_BATCH_SIZE,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand::Locked5Minutes60000Percents,
};

#[tokio::test]
#[mutants::skip]
async fn claim_many_jars() -> Result<()> {
    const INTEREST: u128 = 1_000;

    println!("👷🏽 Claim many jars test");

    set_integration_logs_enabled(false);

    let mut context = prepare_contract(None, [Locked5Minutes60000Percents]).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    context
        .sweat_jar()
        .bulk_create_jars(alice.to_near(), Locked5Minutes60000Percents.id(), INTEREST, 2000)
        .with_user(&manager)
        .await?;

    assert_eq!(
        context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len(),
        2000
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

    for i in 0..10 {
        let withdrawn_summ = context.sweat_jar().withdraw_all(None).with_user(&alice).await?;
        dbg!(&i);
        assert_eq!(withdrawn_summ.jars.len(), JAR_BATCH_SIZE);
        assert_eq!(withdrawn_summ.total_amount.0, INTEREST * JAR_BATCH_SIZE as u128);
    }

    assert_eq!(
        context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len(),
        0
    );

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn restake_many_jars() -> Result<()> {
    const INTEREST: u128 = 1_000;
    const JARS_COUNT: u16 = 2000;

    println!("👷🏽 Restake many jars test");

    set_integration_logs_enabled(false);

    let mut context = prepare_contract(None, [Locked5Minutes60000Percents]).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    context
        .sweat_jar()
        .bulk_create_jars(alice.to_near(), Locked5Minutes60000Percents.id(), INTEREST, JARS_COUNT)
        .with_user(&manager)
        .await?;

    assert_eq!(
        context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len(),
        JARS_COUNT as usize
    );

    context.fast_forward_minutes(5).await?;

    for _ in 0..10 {
        let ClaimedAmountView::Detailed(claimed) =
            context.sweat_jar().claim_total(true.into()).with_user(&alice).await?
        else {
            panic!();
        };
        assert_eq!(claimed.detailed.len(), JAR_BATCH_SIZE);

        let restaked = context.sweat_jar().restake_all(None).with_user(&alice).await?;
        assert_eq!(restaked.len(), JAR_BATCH_SIZE);

        assert_eq!(
            context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len(),
            JARS_COUNT as usize
        );
    }

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;

    let mut ids: Vec<_> = jars.iter().map(|j| j.id.0).collect();

    ids.sort_unstable();

    assert_eq!(ids, (2001..=4000).collect::<Vec<_>>());

    Ok(())
}
