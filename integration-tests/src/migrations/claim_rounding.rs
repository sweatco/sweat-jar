use anyhow::Result;
use integration_utils::{contract_call::set_integration_logs_enabled, misc::ToNear};
use near_sdk::AccountId;
use near_workspaces::types::NearToken;
use sweat_jar_model::api::{
    IntegrationTestMethodsIntegration, MigrationToJarWithRoundingErrorApiIntegration, SweatJarContract,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    migrations::helpers::load_wasm,
    product::RegisterProductCommand,
};

async fn create_state_with_lots_of_jars(accounts: Vec<AccountId>, contract: SweatJarContract<'_>) -> Result<()> {
    for account in accounts {
        contract
            .bulk_create_jars(
                account,
                RegisterProductCommand::Locked12Months12Percents.id(),
                100_000,
                28_000,
            )
            .await?;
    }

    dbg!(contract.total_jars_count().await?);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn migrate_to_claim_roundings() -> Result<()> {
    set_integration_logs_enabled(false);

    let mut context = prepare_contract([
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ])
    .await?;

    let jar_account = context.sweat_jar().contract.as_account().clone();

    let jar_before_rounding = load_wasm("res/sweat_jar_before_rounding.wasm");
    let jar_before_rounding = jar_account.deploy(&jar_before_rounding).await?.into_result()?;
    let jar_before_rounding = SweatJarContract {
        contract: &jar_before_rounding,
    };

    let users_count = 2;

    let mut accounts = Vec::with_capacity(users_count);

    for i in 0..users_count {
        accounts.push(
            context
                .account_with_balance(&format!("user_{i}"), NearToken::from_near(1))
                .await?
                .to_near(),
        );
    }

    create_state_with_lots_of_jars(accounts.clone(), jar_before_rounding).await?;

    let jar_after_rounding = load_wasm("res/sweat_jar.wasm");
    let jar_after_rounding = jar_account.deploy(&jar_after_rounding).await?.into_result()?;
    let jar_after_rounding = SweatJarContract {
        contract: &jar_after_rounding,
    };

    set_integration_logs_enabled(true);

    jar_after_rounding.migrate_to_jars_with_rounding_error(accounts).await?;

    dbg!(jar_after_rounding.total_jars_count().await?);

    Ok(())
}
