use tracing::info;

mod common;
use common::{ft, jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
#[tracing::instrument]
async fn happy_flow() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    info!("running happy flow test");

    let context = prepare_contract([
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ])
    .await?;

    let products = jar::get_products(&context.jar).await?;
    assert_eq!(3, products.len());

    jar::create_jar(
        &context.jar,
        &context.ft,
        &context.alice,
        &RegisterProductCommand::Locked12Months12Percents.id(),
        1_000_000,
    )
    .await?;

    let alice_jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(1_000_000, alice_jars.get_total_principal());

    let mut alice_interest = jar::get_total_interest(&context.jar, context.alice.id()).await?;
    assert_eq!(0, alice_interest.amount.total.0);

    context.fast_forward_hours(1).await?;

    alice_interest = jar::get_total_interest(&context.jar, context.alice.id()).await?;
    assert!(alice_interest.amount.total.0 > 0);

    let claimed_amount = jar::claim_total(&context.jar, &context.alice, None).await?.get_total().0;
    assert!(15 < claimed_amount && claimed_amount < 20);

    let alice_balance = ft::ft_balance_of(&context.ft, context.alice.id()).await?;
    assert_eq!(99_999_999_999_999_999_999_000_000 + claimed_amount, alice_balance);

    Ok(())
}
