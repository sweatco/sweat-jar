use base64::{engine::general_purpose::STANDARD, Engine};

use crate::{
    common::{generate_keypair, prepare_contract, Prepared},
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

    assert_eq!(result.as_str().unwrap(), "1000000");

    context
        .jar_contract
        .set_enabled(&manager, RegisterProductCommand::Locked12Months12Percents.id(), false)
        .await?;

    let result = context
        .jar_contract
        .create_jar(
            &alice,
            product_id.clone(),
            1_000_000,
            context.ft_contract.account().id(),
        )
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

    let (_, verifying_key) = generate_keypair();
    let pk_base64 = STANDARD.encode(verifying_key.as_bytes());

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
        .create_jar(&alice, product_id, 1_000_000, context.ft_contract.account().id())
        .await;

    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .root_cause()
        .to_string()
        .contains("Signature is required"));

    Ok(())
}
