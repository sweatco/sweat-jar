use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::Signer;
use integration_utils::{integration_contract::IntegrationContract, misc::ToNear};
use model::api::{JarApiIntegration, PenaltyApiIntegration, ProductApiIntegration};
use near_sdk::env::sha256;
use serde_json::from_value;

use crate::{
    common::generate_keypair,
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn premium_product() -> anyhow::Result<()> {
    println!("👷🏽 Run test for premium product");

    let (signing_key, verifying_key) = generate_keypair();
    let pk_base64 = STANDARD.encode(verifying_key.as_bytes());

    let mut context = prepare_contract([]).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;

    let register_product_command = RegisterProductCommand::Flexible6Months6Percents;
    let command_json = register_product_command.json_for_premium(pk_base64);

    context
        .sweat_jar()
        .with_user(&manager)
        .register_product(from_value(command_json).unwrap())
        .await?;

    let product_id = register_product_command.id();
    let valid_until = 43_012_170_000_000;
    let amount = 3_000_000;

    let signature = STANDARD.encode(
        signing_key
            .sign(
                sha256(
                    context
                        .sweat_jar()
                        .get_signature_material(&alice, &product_id, valid_until, amount, None)
                        .as_bytes(),
                )
                .as_slice(),
            )
            .to_bytes(),
    );

    let result = context
        .sweat_jar()
        .create_premium_jar(
            &alice,
            product_id.clone(),
            amount,
            signature.to_string(),
            valid_until,
            context.ft_contract().contract().as_account().id(),
        )
        .await?;

    assert_eq!(result.0, amount);

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    let jar_id = jars.first().unwrap().id;

    let jar = context.sweat_jar().get_jar(alice.to_near(), jar_id.clone()).await?;

    assert_eq!(jar.principal.0, amount);
    assert!(!jar.is_penalty_applied);

    context
        .sweat_jar()
        .with_user(&manager)
        .set_penalty(alice.to_near(), jar_id, true)
        .await?;

    let jar = context.sweat_jar().get_jar(alice.to_near(), jar_id).await?;

    assert!(jar.is_penalty_applied);

    let unauthorized_penalty_change = context
        .sweat_jar()
        .with_user(&alice)
        .set_penalty(alice.to_near(), jar_id, true)
        .await;

    assert!(unauthorized_penalty_change.is_err());

    let principal_result = context.sweat_jar().get_principal(vec![jar_id], alice.to_near()).await?;
    assert_eq!(principal_result.total.0, amount);

    let interest_result = context.sweat_jar().get_interest(vec![jar_id], alice.to_near()).await;
    assert!(interest_result.is_ok());

    Ok(())
}
