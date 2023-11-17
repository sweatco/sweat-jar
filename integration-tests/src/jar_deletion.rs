use integration_utils::{integration_contract::IntegrationContract, misc::ToNear};
use model::api::{ClaimApiIntegration, JarApiIntegration, WithdrawApiIntegration};

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn jar_deletion() -> anyhow::Result<()> {
    println!("👷🏽 Run jar deletion test");

    let mut context = prepare_contract([RegisterProductCommand::Locked10Minutes60000Percents]).await?;

    let alice = context.alice().await?;

    context
        .sweat_jar()
        .create_jar(
            &alice,
            RegisterProductCommand::Locked10Minutes60000Percents.id(),
            1_000_000,
            context.ft_contract().contract().as_account().id(),
        )
        .await?;

    let jar_view = context
        .sweat_jar()
        .get_jars_for_account(alice.to_near())
        .await?
        .into_iter()
        .next()
        .unwrap();

    context.fast_forward_minutes(11).await?;

    let withdrawn_amount = context
        .sweat_jar()
        .with_user(&alice)
        .withdraw(jar_view.id, None)
        .await?;
    assert_eq!(withdrawn_amount.withdrawn_amount.0, 1_000_000);

    let alice_interest = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    let claimed_amount = context
        .sweat_jar()
        .with_user(&alice)
        .claim_total(None)
        .await?
        .get_total()
        .0;
    assert_eq!(alice_interest.amount.total.0, claimed_amount);

    let alice_interest = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    assert_eq!(alice_interest.amount.total.0, 0);

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert!(jars.is_empty());

    Ok(())
}
