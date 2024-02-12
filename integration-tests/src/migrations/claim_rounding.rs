use anyhow::Result;
use integration_utils::misc::ToNear;
use near_sdk::AccountId;
use near_workspaces::types::NearToken;
use sweat_jar_model::api::{IntegrationTestMethodsIntegration, SweatJarContract};

use crate::{
    context::{prepare_contract, IntegrationContext},
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
    let mut context = prepare_contract([
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ])
    .await?;

    let users_count = 40;

    let mut accounts = Vec::with_capacity(users_count);

    for i in 0..users_count {
        accounts.push(
            context
                .account_with_balance(&format!("user_{i}"), NearToken::from_near(1))
                .await?
                .to_near(),
        );
    }

    let jar_contract = context.sweat_jar();

    create_state_with_lots_of_jars(accounts, jar_contract).await?;

    Ok(())
}
