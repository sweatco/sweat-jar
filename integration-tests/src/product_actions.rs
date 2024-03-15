use base64::{engine::general_purpose::STANDARD, Engine};
use itertools::Itertools;
use sweat_jar_model::api::ProductApiIntegration;

use crate::{
    common::generate_keypair,
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

    let (_, verifying_key) = generate_keypair();
    let pk_base64 = STANDARD.encode(verifying_key.as_bytes());

    context
        .sweat_jar()
        .set_public_key(
            RegisterProductCommand::Locked12Months12Percents.id(),
            pk_base64.as_bytes().into_iter().copied().collect_vec().into(),
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
