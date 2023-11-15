use integration_utils::{integration_contract::IntegrationContract, misc::ToNear};
use model::api::{ClaimApiIntegration, JarApiIntegration};

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn restake() -> anyhow::Result<()> {
    println!("👷🏽 Run test for restaking");

    let product_command = RegisterProductCommand::Locked10Minutes6Percents;
    let product_id = product_command.id();

    let mut context = prepare_contract([product_command]).await?;

    let alice = context.alice().await?;

    let amount = 1_000_000;
    context
        .sweat_jar()
        .create_jar(
            &alice,
            product_id,
            amount,
            context.ft_contract().contract().as_account().id(),
        )
        .await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    let original_jar_id = jars.first().unwrap().id;

    context.fast_forward_hours(1).await?;

    context.sweat_jar().with_user(&alice).restake(original_jar_id).await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(jars.len(), 2);

    let mut has_original_jar = false;
    let mut has_restaked_jar = false;
    for jar in jars {
        let id = jar.id;

        if id == original_jar_id {
            has_original_jar = true;
            assert_eq!(jar.principal.0, 0);
        } else {
            has_restaked_jar = true;
            assert_eq!(jar.principal.0, amount);
        }
    }

    assert!(has_original_jar);
    assert!(has_restaked_jar);

    context
        .sweat_jar()
        .with_user(&alice)
        .claim_jars(vec![original_jar_id], None, None)
        .await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(jars.len(), 1);

    Ok(())
}
