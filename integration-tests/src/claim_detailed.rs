use nitka::misc::ToNear;
use sweat_jar_model::{
    api::{ClaimApiIntegration, JarApiIntegration, ProductApiIntegration},
    claimed_amount_view::ClaimedAmountView,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
async fn claim_detailed() -> anyhow::Result<()> {
    println!("👷🏽 Run detailed claim test");

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
    let alice_interest = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    assert_eq!(1_000_000, alice_principal.total.0);
    assert_eq!(0, alice_interest.amount.total.0);

    context.fast_forward_hours(1).await?;

    let claimed_details = context.sweat_jar().claim_total(Some(true)).with_user(&alice).await?;

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
