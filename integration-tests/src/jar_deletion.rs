use nitka::misc::ToNear;
use sweat_jar_model::api::{ClaimApiIntegration, JarApiIntegration, WithdrawApiIntegration};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn jar_deletion() -> anyhow::Result<()> {
    println!("👷🏽 Run jar deletion test");

    let mut context = prepare_contract(None, [RegisterProductCommand::Locked10Minutes60000Percents]).await?;

    let alice = context.alice().await?;
    let product_id = RegisterProductCommand::Locked10Minutes60000Percents.id();

    context
        .sweat_jar()
        .create_jar(&alice, product_id.clone(), 1_000_000, &context.ft_contract())
        .await?;

    context.fast_forward_minutes(11).await?;

    let withdrawn_amount = context
        .sweat_jar()
        .withdraw(product_id.clone())
        .with_user(&alice)
        .await?;
    assert_eq!(withdrawn_amount.withdrawn_amount.0, 1_000_000);

    let alice_interest = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    let claimed_amount = context
        .sweat_jar()
        .claim_total(None)
        .with_user(&alice)
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
