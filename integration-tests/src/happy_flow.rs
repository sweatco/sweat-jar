use nitka::misc::ToNear;
use sweat_jar_model::api::{ClaimApiIntegration, JarApiIntegration, ProductApiIntegration};
use sweat_model::FungibleTokenCoreIntegration;

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn happy_flow() -> anyhow::Result<()> {
    println!("👷🏽 Run happy flow test");

    let mut context = prepare_contract(
        None,
        [
            RegisterProductCommand::Locked12Months12Percents,
            RegisterProductCommand::Locked6Months6Percents,
            RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
        ],
    )
    .await?;

    let alice = context.alice().await?;

    let products = context.sweat_jar().get_products().await?;
    assert_eq!(3, products.len());

    context
        .sweat_jar()
        .create_jar(
            &alice,
            RegisterProductCommand::Locked12Months12Percents.id(),
            1_000_000,
            &context.ft_contract(),
        )
        .await?;

    let alice_principal = context.sweat_jar().get_total_principal(alice.to_near()).await?;
    let mut alice_interest = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    assert_eq!(1_000_000, alice_principal.total.0);
    assert_eq!(0, alice_interest.amount.total.0);

    context.fast_forward_hours(1).await?;

    alice_interest = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    assert!(alice_interest.amount.total.0 > 0);

    let claimed_amount = context
        .sweat_jar()
        .claim_total(None)
        .with_user(&alice)
        .await?
        .get_total()
        .0;
    assert!(15 < claimed_amount && claimed_amount < 20);

    let alice_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?.0;
    assert_eq!(99_000_000 + claimed_amount, alice_balance);

    Ok(())
}
