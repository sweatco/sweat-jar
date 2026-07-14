mod common;

use anyhow::Result;
use common::{ft, jar, prepare::prepare_contract, product::RegisterProductCommand};

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
    let mut alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    assert_eq!(1_000_000, alice_principal.total.0);
    assert_eq!(0, alice_interest.amount.total.0);

    context.fast_forward_hours(1).await?;

    alice_interest = jar::get_total_interest(&context.jar, alice.id()).await?;
    assert!(alice_interest.amount.total.0 > 0);

    let claimed_amount = jar::claim_total(&context.jar, alice, None).await?.get_total().0;
    assert!(15 < claimed_amount && claimed_amount < 20);

    let alice_balance = ft::ft_balance_of(&context.ft, alice.id()).await?;
    assert_eq!(99_000_000 + claimed_amount, alice_balance);

    Ok(())
}
