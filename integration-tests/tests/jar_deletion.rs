mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
async fn jar_deletion() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked10Minutes60000Percents]).await?;
    let alice = &context.alice;

    jar::create_jar(
        &context.ft,
        &context.jar,
        alice,
        &RegisterProductCommand::Locked10Minutes60000Percents.id(),
        1_000_000,
    )
    .await?
    .into_result()?;

    let jar_view = jar::get_jars_for_account(&context.jar, alice.id())
        .await?
        .into_iter()
        .next()
        .unwrap();

    context.fast_forward_minutes(11).await?;

    let withdrawn_amount = jar::withdraw(&context.jar, alice, jar_view.id, None).await?;
    assert_eq!(withdrawn_amount.withdrawn_amount.0, 1_000_000);

    let alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    let claimed_amount = jar::claim_total(&context.jar, alice, None).await?.get_total().0;
    assert_eq!(alice_interest.amount.total.0, claimed_amount);

    let alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    assert_eq!(alice_interest.amount.total.0, 0);

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    assert!(jars.is_empty());

    Ok(())
}
