use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::Signer;
use near_sdk::env::sha256;

use crate::{
    common::{generate_keypair, prepare_contract, Prepared},
    product::RegisterProductCommand,
};

#[tokio::test]
async fn premium_product() -> anyhow::Result<()> {
    println!("👷🏽 Run test for premium product");

    let (signing_key, verifying_key) = generate_keypair();
    let pk_base64 = STANDARD.encode(verifying_key.as_bytes());

    let Prepared {
        context,
        manager,
        alice,
        fee_account: _,
    } = prepare_contract([]).await?;

    let register_product_command = RegisterProductCommand::Flexible6Months6Percents;
    let command_json = register_product_command.json_for_premium(pk_base64);

    context.jar_contract.register_product(&manager, command_json).await?;

    let product_id = register_product_command.id();
    let valid_until = 43_012_170_000_000;
    let amount = 3_000_000;

    let signature = STANDARD.encode(
        signing_key
            .sign(
                sha256(
                    context
                        .get_signature_material(&alice, &product_id, valid_until, amount, None)
                        .as_bytes(),
                )
                .as_slice(),
            )
            .to_bytes(),
    );

    let result = context
        .jar_contract
        .create_premium_jar(
            &alice,
            product_id.clone(),
            amount,
            signature.to_string(),
            valid_until,
            context.ft_contract.account().id(),
        )
        .await?;

    assert_eq!(result.as_str().unwrap(), amount.to_string());

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    let jar_id = jars
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    let jar = context
        .jar_contract
        .get_jar(alice.id().to_string(), jar_id.clone())
        .await?;

    assert_eq!(
        jar.as_object().unwrap().get("principal").unwrap().as_str().unwrap(),
        amount.to_string()
    );
    assert!(!jar
        .as_object()
        .unwrap()
        .get("is_penalty_applied")
        .unwrap()
        .as_bool()
        .unwrap());

    context
        .jar_contract
        .set_penalty(&manager, alice.id(), &jar_id.clone(), true)
        .await?;

    let jar = context
        .jar_contract
        .get_jar(alice.id().to_string(), jar_id.clone())
        .await?;

    assert!(jar
        .as_object()
        .unwrap()
        .get("is_penalty_applied")
        .unwrap()
        .as_bool()
        .unwrap());

    let unauthorized_penalty_change = context
        .jar_contract
        .set_penalty(&alice, &alice.id().to_string(), &jar_id.clone(), true)
        .await;

    assert!(unauthorized_penalty_change.is_err());

    let principal_result = context.jar_contract.get_principal(&alice, vec![jar_id.clone()]).await?;
    assert_eq!(
        principal_result
            .as_object()
            .unwrap()
            .get("total")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        amount.to_string()
    );

    let interest_result = context.jar_contract.get_interest(&alice, vec![jar_id]).await;
    assert!(interest_result.is_ok());

    Ok(())
}
