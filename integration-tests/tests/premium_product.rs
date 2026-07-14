mod common;

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use common::{generate_keypair, jar, prepare::prepare_contract, product::RegisterProductCommand};
use ed25519_dalek::Signer;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn premium_product() -> Result<()> {
    let (signing_key, verifying_key) = generate_keypair();
    let pk_base64 = STANDARD.encode(verifying_key.as_bytes());

    let context = prepare_contract([]).await?;
    let alice = &context.alice;
    let manager = &context.manager;

    let register_product_command = RegisterProductCommand::Flexible6Months6Percents;
    jar::register_product(
        &context.jar,
        manager,
        register_product_command.json_for_premium(pk_base64),
    )
    .await?;

    let product_id = register_product_command.id();
    let valid_until = 43_012_170_000_000;
    let amount = 3_000_000;

    let material = jar::signature_material(&context.jar, alice.id(), &product_id, valid_until, amount, None);
    let hash = Sha256::digest(material.as_bytes());
    let signature = STANDARD.encode(signing_key.sign(&hash).to_bytes());

    let used = jar::used_amount(
        jar::create_premium_jar(
            &context.ft,
            &context.jar,
            alice,
            &product_id,
            amount,
            &signature,
            valid_until,
        )
        .await?,
    )?;
    assert_eq!(used, amount);

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    let jar_id = jars.first().unwrap().id;

    let jar_view = jar::get_jar(&context.jar, alice.id(), jar_id).await?;
    assert_eq!(jar_view.principal.0, amount);
    assert!(!jar_view.is_penalty_applied);

    jar::set_penalty(&context.jar, manager, alice.id(), jar_id, true)
        .await?
        .into_result()?;

    let jar_view = jar::get_jar(&context.jar, alice.id(), jar_id).await?;
    assert!(jar_view.is_penalty_applied);

    let unauthorized_penalty_change = jar::set_penalty(&context.jar, alice, alice.id(), jar_id, true).await?;
    assert!(unauthorized_penalty_change.into_result().is_err());

    let principal_result = jar::get_principal(&context.jar, alice.id(), vec![jar_id]).await?;
    assert_eq!(principal_result.total.0, amount);

    let interest_result = jar::get_interest(&context.jar, alice.id(), vec![jar_id]).await;
    assert!(interest_result.is_ok());

    Ok(())
}
