mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};
use sweat_jar_model::claimed_amount_view::ClaimedAmountView;

#[tokio::test]
async fn claim_detailed() -> Result<()> {
    let context = prepare_contract([
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ])
    .await?;
    let alice = &context.alice;

    let products = jar::get_products(&context.jar).await?;
    assert_eq!(3, products.len());

    jar::create_jar(
        &context.ft,
        &context.jar,
        alice,
        &RegisterProductCommand::Locked12Months12Percents.id(),
        1_000_000,
    )
    .await?
    .into_result()?;

    let alice_principal = jar::get_total_principal(&context.jar, alice.id()).await?;
    let alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    assert_eq!(1_000_000, alice_principal.total.0);
    assert_eq!(0, alice_interest.amount.total.0);

    context.fast_forward_hours(1).await?;

    let ClaimedAmountView::Detailed(claimed_details) = jar::claim_total(&context.jar, alice, Some(true)).await? else {
        panic!("Expected detailed claim result")
    };

    let claimed_amount = claimed_details.total.0;

    assert!(15 < claimed_amount && claimed_amount < 20);
    assert_eq!(
        claimed_amount,
        claimed_details.detailed.values().map(|item| item.0).sum()
    );

    Ok(())
}
