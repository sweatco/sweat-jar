mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand::Locked5Minutes60000Percents};
use sweat_jar_model::{claimed_amount_view::ClaimedAmountView, JAR_BATCH_SIZE};

#[tokio::test]
async fn claim_many_jars() -> Result<()> {
    const INTEREST: u128 = 1_000;

    let context = prepare_contract([Locked5Minutes60000Percents]).await?;
    let alice = &context.alice;

    jar::bulk_create_jars(
        &context.jar,
        &context.manager,
        alice.id(),
        &Locked5Minutes60000Percents.id(),
        INTEREST,
        2000,
    )
    .await?;

    assert_eq!(jar::get_jars_for_account(&context.jar, alice.id()).await?.len(), 2000);

    context.fast_forward_minutes(5).await?;

    let claimed = jar::claim_total(&context.jar, alice, Some(true)).await?;
    let batch_claim_sum = claimed.get_total().0;

    assert_eq!(
        batch_claim_sum * 9,
        jar::get_total_interest(&context.jar, alice.id()).await?.amount.total.0
    );

    for i in 1..10 {
        let claimed = jar::claim_total(&context.jar, alice, Some(true)).await?;
        assert_eq!(claimed.get_total().0, batch_claim_sum);

        assert_eq!(
            batch_claim_sum * (9 - i),
            jar::get_total_interest(&context.jar, alice.id()).await?.amount.total.0
        );
    }

    assert_eq!(
        jar::get_total_interest(&context.jar, alice.id()).await?.amount.total.0,
        0
    );
    assert_eq!(jar::get_jars_for_account(&context.jar, alice.id()).await?.len(), 2000);

    for _ in 0..10 {
        let withdrawn_sum = jar::withdraw_all(&context.jar, alice).await?;
        assert_eq!(withdrawn_sum.jars.len(), JAR_BATCH_SIZE);
        assert_eq!(withdrawn_sum.total_amount.0, INTEREST * JAR_BATCH_SIZE as u128);
    }

    assert_eq!(jar::get_jars_for_account(&context.jar, alice.id()).await?.len(), 0);

    Ok(())
}

#[tokio::test]
async fn restake_many_jars() -> Result<()> {
    const INTEREST: u128 = 1_000;
    const JARS_COUNT: u16 = 2000;

    let context = prepare_contract([Locked5Minutes60000Percents]).await?;
    let alice = &context.alice;

    jar::bulk_create_jars(
        &context.jar,
        &context.manager,
        alice.id(),
        &Locked5Minutes60000Percents.id(),
        INTEREST,
        JARS_COUNT,
    )
    .await?;

    assert_eq!(
        jar::get_jars_for_account(&context.jar, alice.id()).await?.len(),
        JARS_COUNT as usize
    );

    context.fast_forward_minutes(5).await?;

    for _ in 0..10 {
        let ClaimedAmountView::Detailed(claimed) = jar::claim_total(&context.jar, alice, Some(true)).await? else {
            panic!("Expected detailed claim result");
        };
        assert_eq!(claimed.detailed.len(), JAR_BATCH_SIZE);

        let restaked = jar::restake_all(&context.jar, alice).await?;
        assert_eq!(restaked.len(), JAR_BATCH_SIZE);

        assert_eq!(
            jar::get_jars_for_account(&context.jar, alice.id()).await?.len(),
            JARS_COUNT as usize
        );
    }

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;

    let mut ids: Vec<_> = jars.iter().map(|j| j.id.0).collect();
    ids.sort_unstable();

    assert_eq!(ids, (2001..=4000).collect::<Vec<_>>());

    Ok(())
}
