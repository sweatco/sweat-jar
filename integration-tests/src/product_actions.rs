use sweat_jar_model::{api::ProductApiIntegration, signer::test_utils::MessageSigner};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn product_actions() -> anyhow::Result<()> {
    println!("👷🏽 Run test for product actions");

    let mut context = prepare_contract(None, [RegisterProductCommand::Locked12Months12Percents]).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    let result = context
        .sweat_jar()
        .create_jar(&alice, product_id.clone(), 1_000_000, &context.ft_contract())
        .await?;

    assert_eq!(result.0, 1_000_000);

    context
        .sweat_jar()
        .set_enabled(RegisterProductCommand::Locked12Months12Percents.id(), false)
        .with_user(&manager)
        .await?;

    let result = context
        .sweat_jar()
        .create_jar(&alice, product_id.clone(), 1_000_000, &context.ft_contract())
        .result()
        .await;

    assert!(format!("{result:?}")
        .contains("Smart contract panicked: It's not possible to create new jars for this product"));

    context
        .sweat_jar()
        .set_enabled(RegisterProductCommand::Locked12Months12Percents.id(), true)
        .with_user(&manager)
        .await?;

    let signer = MessageSigner::new();
    context
        .sweat_jar()
        .set_public_key(
            RegisterProductCommand::Locked12Months12Percents.id(),
            signer.public_key().into(),
        )
        .with_user(&manager)
        .await?;

    let result = context
        .sweat_jar()
        .create_jar(&alice, product_id, 1_000_000, &context.ft_contract())
        .result()
        .await;

    assert!(format!("{result:?}").contains("Smart contract panicked: Signature is required"));

    Ok(())
}
