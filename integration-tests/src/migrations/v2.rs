use nitka::{json, misc::ToNear, near_sdk::serde_json::Value};
use sweat_jar_model::api::{IntegrationTestMethodsIntegration, MigrationToV2Integration};

use crate::{
    context::{prepare_contract, IntegrationContext, SWEAT_JAR_V2},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn migrate_to_v2() -> anyhow::Result<()> {
    println!("👷🏽 Migrate to v2");

    let original_products = vec![
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked5Minutes60000Percents,
        RegisterProductCommand::Locked10Minutes60000Percents,
    ];

    let mut context = prepare_contract(None, original_products.clone()).await?;
    let alice = context.alice().await?;
    let manager = context.manager().await?;
    let fee_account = context.fee().await?;

    context
        .sweat_jar()
        .bulk_create_jars(
            alice.to_near(),
            RegisterProductCommand::Locked12Months12Percents.id(),
            3 * 10u128.pow(18),
            500,
        )
        .with_user(&manager)
        .await?;
    context
        .sweat_jar()
        .bulk_create_jars(
            alice.to_near(),
            RegisterProductCommand::Locked6Months6Percents.id(),
            2 * 10u128.pow(18),
            500,
        )
        .with_user(&manager)
        .await?;
    context
        .sweat_jar()
        .bulk_create_jars(
            alice.to_near(),
            RegisterProductCommand::Locked5Minutes60000Percents.id(),
            5 * 10u128.pow(18),
            500,
        )
        .with_user(&manager)
        .await?;
    context
        .sweat_jar()
        .bulk_create_jars(
            alice.to_near(),
            RegisterProductCommand::Locked10Minutes60000Percents.id(),
            7 * 10u128.pow(18),
            500,
        )
        .with_user(&manager)
        .await?;

    println!("👷🏽 Init v2 contract");
    let v2_contract = context.contracts.get(SWEAT_JAR_V2).unwrap();
    let result = v2_contract
        .call("init")
        .args_json(json!({
            "token_account_id": context.ft_contract().contract.as_account().to_near(),
            "fee_account_id": fee_account.to_near(),
            "manager": manager.to_near(),
            "previous_version_account_id": context.sweat_jar().contract.as_account().to_near(),
        }))
        .max_gas()
        .transact()
        .await?;
    println!("Initialization is successful: {:?}", result.is_success());

    let result = context
        .sweat_jar()
        .migrate_products()
        .with_user(&manager)
        .result()
        .await;
    assert!(result.is_ok(), "🚨 Products migration failed: {:?}", result);

    let products: Vec<Value> = v2_contract.call("get_products").view().await?.json()?;
    assert_eq!(original_products.len(), products.len());

    context.sweat_jar().migrate_account().with_user(&alice).await?;

    Ok(())
}
