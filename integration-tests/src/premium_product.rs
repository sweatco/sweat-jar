use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::Signer;
use nitka::{misc::ToNear, near_sdk::serde_json::from_value};
use sha2::{Digest, Sha256};
use sweat_jar_model::{
    api::{JarApiIntegration, PenaltyApiIntegration, ProductApiIntegration},
    TokenAmount,
};

use crate::{
    common::{generate_keypair, total_principal},
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn premium_product() -> anyhow::Result<()> {
    println!("👷🏽 Run test for premium product");

    let (signing_key, verifying_key) = generate_keypair();
    let pk_base64 = STANDARD.encode(verifying_key.as_bytes());

    let mut context = prepare_contract(None, []).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;

    let register_product_command = RegisterProductCommand::Flexible6Months6Percents;
    let command_json = register_product_command.json_for_premium(pk_base64);

    context
        .sweat_jar()
        .register_product(from_value(command_json).unwrap())
        .with_user(&manager)
        .await?;

    let product_id = register_product_command.id();
    let valid_until = 43_012_170_000_000;
    let amount = 3_000_000;

    let hash = Sha256::digest(
        context
            .sweat_jar()
            .get_signature_material(&alice, &product_id, valid_until, amount, 0)
            .as_bytes(),
    );

    let signature = STANDARD.encode(signing_key.sign(hash.as_slice()).to_bytes());

    let result = context
        .sweat_jar()
        .create_premium_jar(
            &alice,
            product_id.clone(),
            amount,
            signature.to_string(),
            valid_until,
            &context.ft_contract(),
        )
        .await?;

    assert_eq!(result.0, amount);

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(jars.first().unwrap().principal.0, amount);

    let is_penalty_applied = context.sweat_jar().is_penalty_applied(alice.to_near()).await?;
    assert!(!is_penalty_applied);

    context
        .sweat_jar()
        .set_penalty(alice.to_near(), true)
        .with_user(&manager)
        .await?;

    let is_penalty_applied = context.sweat_jar().is_penalty_applied(alice.to_near()).await?;
    assert!(is_penalty_applied);

    let unauthorized_penalty_change = context
        .sweat_jar()
        .set_penalty(alice.to_near(), true)
        .with_user(&alice)
        .await;

    assert!(unauthorized_penalty_change.is_err());

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    let total_principal: TokenAmount = total_principal(&jars);
    assert_eq!(total_principal, amount);

    let interest_result = context.sweat_jar().get_total_interest(alice.to_near()).await;
    assert!(interest_result.is_ok());

    Ok(())
}
