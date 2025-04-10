use nitka::{json, misc::ToNear};
use sweat_jar_model::api::{IntegrationTestMethodsIntegration, MigrationToV2Integration};

use crate::{
    context::{prepare_contract, IntegrationContext, SWEAT_JAR_V2},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn migrate_to_v2() -> anyhow::Result<()> {
    println!("👷🏽 Migrate to v2");

    let mut context = prepare_contract(
        None,
        [
            RegisterProductCommand::Locked12Months12Percents,
            RegisterProductCommand::Locked6Months6Percents,
            RegisterProductCommand::Locked5Minutes60000Percents,
            RegisterProductCommand::Locked10Minutes60000Percents,
        ],
    )
    .await?;
    let alice = context.alice().await?;
    let manager = context.manager().await?;

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
            "fee_account_id": context.fee().await?.to_near(),
            "manager": manager.to_near(),
            "previous_version_account_id": context.sweat_jar().contract.as_account().to_near(),
        }))
        .max_gas()
        .transact()
        .await?;
    println!("Initialization is successful: {:?}", result.is_success());

    context.sweat_jar().migrate_account().with_user(&alice).await?;

    Ok(())
}
