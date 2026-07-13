use sweat_jar_model::data::claim::ClaimedAmountView;
use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
#[tracing::instrument]
async fn claim_detailed() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    info!("detailed claim test");

    let context = prepare_contract([
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ])
    .await?;

    let target_principal = 1_000_000;

    let products = jar::get_products(&context.jar).await?;
    assert_eq!(3, products.len());

    jar::create_jar(
        &context.jar,
        &context.ft,
        &context.alice,
        &RegisterProductCommand::Locked12Months12Percents.id(),
        target_principal,
    )
    .await?;

    let alice_principal = jar::get_jars_for_account(&context.jar, context.alice.id())
        .await?
        .get_total_principal();
    let alice_interest = jar::get_total_interest(&context.jar, context.alice.id()).await?;
    assert_eq!(target_principal, alice_principal);
    assert_eq!(0, alice_interest.amount.total.0);

    context.fast_forward_hours(1).await?;

    let claimed_details = jar::claim_total(&context.jar, &context.alice, Some(true)).await?;

    let ClaimedAmountView::Detailed(claimed_details) = claimed_details else {
        panic!()
    };

    let claimed_amount = claimed_details.total.0;

    assert!(15 < claimed_amount && claimed_amount < 20);
    assert_eq!(
        claimed_amount,
        claimed_details.detailed.values().map(|item| item.0).sum()
    );

    Ok(())
}
