mod common;

use anyhow::Result;
use common::{ft, interest, jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
async fn happy_flow() -> Result<()> {
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
    let mut alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    assert_eq!(principal, alice_principal.total.0);
    assert_eq!(0, alice_interest.amount.total.0);

    // Just enough sandbox time for interest to accrue measurably; the assertions below are
    // anchored to the actually elapsed on-chain time (see common::interest), not this nominal
    // duration, so they don't need re-deriving if this gets tuned again.
    context.fast_forward_minutes(10).await?;

    alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    let elapsed_ms = jar::block_timestamp_ms(&context.jar).await? - start_ms;
    let expected = interest::expected_interest(elapsed_ms, principal, apy);
    assert!(expected > 0, "test setup produced no measurable interest");
    assert!(
        alice_interest.amount.total.0.abs_diff(expected) <= expected / 10 + 1,
        "accrued interest {} too far from expected {expected} (elapsed {elapsed_ms} ms)",
        alice_interest.amount.total.0
    );

    let claimed_amount = jar::claim_total(&context.jar, alice, None).await?.get_total().0;
    assert!(
        claimed_amount.abs_diff(expected) <= expected / 10 + 2,
        "claimed {claimed_amount} too far from expected {expected} (elapsed {elapsed_ms} ms)"
    );

    let alice_balance = ft::ft_balance_of(&context.ft, alice.id()).await?;
    assert_eq!(99_000_000 + claimed_amount, alice_balance);

    Ok(())
}
