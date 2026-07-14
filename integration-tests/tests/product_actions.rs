mod common;

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use common::{generate_keypair, jar, panic::PanicFinder, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
async fn product_actions() -> Result<()> {
    let context = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;
    let alice = &context.alice;
    let manager = &context.manager;

    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    let amount = jar::used_amount(jar::create_jar(&context.ft, &context.jar, alice, &product_id, 1_000_000).await?)?;
    assert_eq!(amount, 1_000_000);

    jar::set_enabled(&context.jar, manager, &product_id, false).await?;

    let result = jar::create_jar(&context.ft, &context.jar, alice, &product_id, 1_000_000)
        .await?
        .into_result();
    assert!(result.has_panic("It's not possible to create new jars for this product"));

    jar::set_enabled(&context.jar, manager, &product_id, true).await?;

    let (_, verifying_key) = generate_keypair();
    let pk_base64 = STANDARD.encode(verifying_key.as_bytes());

    jar::set_public_key(&context.jar, manager, &product_id, &pk_base64).await?;

    let result = jar::create_jar(&context.ft, &context.jar, alice, &product_id, 1_000_000)
        .await?
        .into_result();
    assert!(result.has_panic("Signature is required"));

    Ok(())
}
