mod common;

use anyhow::Result;
use common::{interest, jar, prepare::prepare_contract, product::RegisterProductCommand};
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

    let product_id = RegisterProductCommand::Locked12Months12Percents.id();
    let apy = interest::constant_apy(&products, &product_id);
    let principal = 1_000_000u128;

    let start_ms = jar::block_timestamp_ms(&context.jar).await?;

    jar::create_jar(&context.ft, &context.jar, alice, &product_id, principal)
        .await?
        .into_result()?;

    let alice_principal = jar::get_total_principal(&context.jar, alice.id()).await?;
    let alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    assert_eq!(principal, alice_principal.total.0);
    assert_eq!(0, alice_interest.amount.total.0);

    // See happy_flow.rs: kept short since the assertions below are anchored to the
    // actually elapsed on-chain time, not this nominal duration.
    context.fast_forward_minutes(10).await?;

    let elapsed_ms = jar::block_timestamp_ms(&context.jar).await? - start_ms;
    let expected = interest::expected_interest(elapsed_ms, principal, apy);
    assert!(expected > 0, "test setup produced no measurable interest");

    let ClaimedAmountView::Detailed(claimed_details) = jar::claim_total(&context.jar, alice, Some(true)).await? else {
        panic!("Expected detailed claim result")
    };

    let claimed_amount = claimed_details.total.0;

    assert!(
        claimed_amount.abs_diff(expected) <= expected / 10 + 2,
        "claimed {claimed_amount} too far from expected {expected} (elapsed {elapsed_ms} ms)"
    );
    assert_eq!(
        claimed_amount,
        claimed_details.detailed.values().map(|item| item.0).sum()
    );

    Ok(())
}
