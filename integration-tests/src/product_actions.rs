use ed25519_dalek::{SigningKey, VerifyingKey};
use near_sdk::json_types::Base64VecU8;
use rand::rngs::OsRng;

use crate::{
    common::{prepare_contract, Prepared},
    product::RegisterProductCommand,
};

#[tokio::test]
async fn product_actions() -> anyhow::Result<()> {
    println!("👷🏽 Run test for product actions");

    let Prepared {
        context,
        manager,
        alice,
        fee_account: _,
    } = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;

    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    let result = context
        .jar_contract
        .create_jar(
            &alice,
            product_id.clone(),
            1_000_000,
            context.ft_contract.account().id(),
        )
        .await?;

    context
        .jar_contract
        .set_enabled(&manager, RegisterProductCommand::Locked12Months12Percents.id(), false)
        .await?;

    let result = context
        .jar_contract
        .create_jar(&alice, product_id.clone(), 100, context.ft_contract.account().id())
        .await;

    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .root_cause()
        .to_string()
        .contains("It's not possible to create new jars for this product"));

    context
        .jar_contract
        .set_enabled(&manager, RegisterProductCommand::Locked12Months12Percents.id(), true)
        .await?;

    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key: VerifyingKey = VerifyingKey::from(&signing_key);
    let pk_base64 = serde_json::to_string(&Base64VecU8(verifying_key.as_bytes().to_vec())).unwrap();

    context
        .jar_contract
        .set_public_key(
            &manager,
            RegisterProductCommand::Locked12Months12Percents.id(),
            pk_base64,
        )
        .await?;

    let result = context
        .jar_contract
        .create_jar(&alice, product_id, 100, context.ft_contract.account().id())
        .await;

    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .root_cause()
        .to_string()
        .contains("It's not possible to create new jars for this product"));

    Ok(())
}
