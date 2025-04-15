use nitka::misc::ToNear;
use sweat_jar_model::{
    api::{JarApiIntegration, PenaltyApiIntegration, ProductApiIntegration},
    data::deposit::DepositMessage,
    signer::test_utils::MessageSigner,
    TokenAmount,
};

use crate::{
    common::total_principal,
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn premium_product() -> anyhow::Result<()> {
    println!("👷🏽 Run test for premium product");

    let signer = MessageSigner::new();
    let mut context = prepare_contract(None, []).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;

    let product = RegisterProductCommand::Flexible6Months6Percents
        .get()
        .with_public_key(Some(signer.public_key()));

    context
        .sweat_jar()
        .register_product(product.clone())
        .with_user(&manager)
        .await?;

    let product_id = &product.id;
    let valid_until = 55_012_170_000_000;
    let amount = 3_000_000;
    let deposit_message = DepositMessage::new(
        context.sweat_jar().contract.as_account().id(),
        alice.id(),
        product_id,
        amount,
        valid_until,
        0,
    );
    let signature = signer.sign(deposit_message.as_str());

    let result = context
        .sweat_jar()
        .create_premium_jar(
            &alice,
            product_id.clone(),
            amount,
            signature.into(),
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
