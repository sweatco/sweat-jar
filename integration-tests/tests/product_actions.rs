use sweat_jar_model::signer::test_utils::MessageSigner;
use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
#[tracing::instrument]
async fn product_actions() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    info!("product actions test");

    let context = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;

    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    let result = jar::create_jar(&context.jar, &context.ft, &context.alice, &product_id, 1_000_000).await?;
    assert_eq!(result, 1_000_000);

    jar::set_enabled(&context.jar, &context.manager, &product_id, false).await?;

    let result = jar::create_jar_raw(&context.jar, &context.ft, &context.alice, &product_id, 1_000_000).await?;
    assert!(format!("{result:?}")
        .contains("Smart contract panicked: It's not possible to create new jars for this product"));

    jar::set_enabled(&context.jar, &context.manager, &product_id, true).await?;

    let signer = MessageSigner::new();
    jar::set_public_key(&context.jar, &context.manager, &product_id, signer.public_key().into()).await?;

    let result = jar::create_jar_raw(&context.jar, &context.ft, &context.alice, &product_id, 1_000_000).await?;
    assert!(format!("{result:?}").contains("Smart contract panicked: Signature is required"));

    Ok(())
}
