use serde_json::json;
use sweat_jar_model::{
    data::{
        deposit::{DepositMessage, Purpose},
        product::Product,
    },
    signer::test_utils::MessageSigner,
    TokenAmount, MS_IN_DAY,
};
use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
#[tracing::instrument]
async fn premium_product() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    info!("premium product test");

    let signer = MessageSigner::new();
    let context = prepare_contract([]).await?;

    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..RegisterProductCommand::Flexible6Months6Percents.get()
    };

    jar::register_product(&context.jar, &context.manager, product.clone()).await?;

    context
        .manager
        .call(context.jar.id(), "set_feature_enabled")
        .args_json(json!({ "account_id": context.alice.id(), "feature": "increased_apy", "enabled": true }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let product_id = &product.id;
    // Signed tickets must satisfy the on-chain upper bound on `valid_until`
    // (at most 7 real days from now), so anchor it to the chain's clock.
    let valid_until = jar::block_timestamp_ms(&context.jar).await? + MS_IN_DAY;
    let amount = 3_000_000;
    let deposit_message = DepositMessage::new(
        Purpose::Deposit,
        context.jar.id(),
        context.alice.id(),
        product_id,
        amount,
        valid_until,
        0,
    );
    let signature = signer.sign(deposit_message.as_str());

    let result = jar::create_premium_jar(
        &context.jar,
        &context.ft,
        &context.alice,
        product_id,
        amount,
        signature.into(),
        valid_until,
    )
    .await?;
    assert_eq!(result, amount);

    let jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(jars.get_first_deposit().unwrap().principal(), amount);

    let is_penalty_applied = jar::is_penalty_applied(&context.jar, context.alice.id()).await?;
    assert!(!is_penalty_applied);

    jar::set_penalty(&context.jar, &context.manager, context.alice.id(), true).await?;

    let is_penalty_applied = jar::is_penalty_applied(&context.jar, context.alice.id()).await?;
    assert!(is_penalty_applied);

    let unauthorized_penalty_change = jar::set_penalty(&context.jar, &context.alice, context.alice.id(), true).await;
    assert!(unauthorized_penalty_change.is_err());

    let jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    let total_principal: TokenAmount = jars.get_total_principal();
    assert_eq!(total_principal, amount);

    let interest_result = jar::get_total_interest(&context.jar, context.alice.id()).await;
    assert!(interest_result.is_ok());

    Ok(())
}
