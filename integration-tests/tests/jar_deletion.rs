use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
#[tracing::instrument]
async fn jar_deletion() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    info!("jar deletion test");

    let context = prepare_contract([RegisterProductCommand::Locked10Minutes60000Percents]).await?;

    let product_id = RegisterProductCommand::Locked10Minutes60000Percents.id();

    jar::create_jar(&context.jar, &context.ft, &context.alice, &product_id, 1_000_000).await?;

    context.fast_forward_minutes(11).await?;

    let withdrawn_amount = jar::withdraw(&context.jar, &context.alice, &product_id).await?;
    assert_eq!(withdrawn_amount.withdrawn_amount.0, 1_000_000);

    let alice_interest = jar::get_total_interest(&context.jar, context.alice.id()).await?;
    let claimed_amount = jar::claim_total(&context.jar, &context.alice, None).await?.get_total().0;
    assert_eq!(alice_interest.amount.total.0, claimed_amount);

    let alice_interest = jar::get_total_interest(&context.jar, context.alice.id()).await?;
    assert_eq!(alice_interest.amount.total.0, 0);

    let jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert!(jars.is_empty());

    Ok(())
}
