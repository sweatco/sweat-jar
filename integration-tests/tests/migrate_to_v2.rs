mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};
use serde_json::{json, Value};

#[tokio::test]
async fn migrate_to_v2() -> Result<()> {
    let original_products = [
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked5Minutes60000Percents,
        RegisterProductCommand::Locked10Minutes60000Percents,
    ];

    let context = prepare_contract(original_products).await?;
    let alice = &context.alice;
    let manager = &context.manager;

    for (product, principal) in [
        (RegisterProductCommand::Locked12Months12Percents, 3 * 10u128.pow(18)),
        (RegisterProductCommand::Locked6Months6Percents, 2 * 10u128.pow(18)),
        (RegisterProductCommand::Locked5Minutes60000Percents, 5 * 10u128.pow(18)),
        (RegisterProductCommand::Locked10Minutes60000Percents, 7 * 10u128.pow(18)),
    ] {
        jar::bulk_create_jars(&context.jar, manager, alice.id(), &product.id(), principal, 500).await?;
    }

    context
        .jar_v2
        .call("init")
        .args_json(json!({
            "token_account_id": context.ft.id(),
            "fee_account_id": context.fee.id(),
            "manager": manager.id(),
            "previous_version_account_id": context.jar.id(),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let result = jar::migrate_products(&context.jar, manager).await?.into_result();
    assert!(result.is_ok(), "🚨 Products migration failed: {result:?}");

    let products: Vec<Value> = context.jar_v2.view("get_products").await?.json()?;
    assert_eq!(original_products.len(), products.len());

    jar::migrate_account(&context.jar, alice).await?.into_result()?;

    Ok(())
}
