use anyhow::Result;
use integration_utils::{contract_call::set_integration_logs_enabled, misc::ToNear};
use near_sdk::AccountId;
use near_workspaces::types::NearToken;
use sweat_jar_model::api::{
    IntegrationTestMethodsIntegration, JarApiIntegration, MigrationToClaimRemainderIntegration, SweatJarContract,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    migrations::helpers::load_wasm,
    product::RegisterProductCommand,
};

#[tokio::test]
#[ignore]
async fn migrate_to_claim_roundings() -> Result<()> {
    set_integration_logs_enabled(false);

    let jar_before_rounding = load_wasm("res/sweat_jar_before_rounding.wasm");

    let mut context = prepare_contract(
        jar_before_rounding.into(),
        [
            RegisterProductCommand::Locked12Months12Percents,
            RegisterProductCommand::Locked6Months6Percents,
            RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
        ],
    )
    .await?;

    let jar_account = context.sweat_jar().contract.as_account().clone();

    let users_count = 50;

    let mut accounts = Vec::with_capacity(users_count);

    for i in 0..users_count {
        accounts.push(
            context
                .account_with_balance(&format!("user_{i}"), NearToken::from_near(1))
                .await?
                .to_near(),
        );
    }

    for accs in accounts.chunks(20) {
        context
            .sweat_jar()
            .bulk_create_jars(
                accs.to_vec(),
                RegisterProductCommand::Locked12Months12Percents.id(),
                100_000,
                1_000,
            )
            .await?;
    }

    let jar_after_rounding = load_wasm("res/sweat_jar.wasm");
    let jar_after_rounding = jar_account.deploy(&jar_after_rounding).await?.into_result()?;
    let jar_after_rounding = SweatJarContract {
        contract: &jar_after_rounding,
    };

    set_integration_logs_enabled(true);

    jar_after_rounding.migrate_state_to_claim_remainder().await?;

    for accs in accounts.chunks(20) {
        jar_after_rounding
            .migrate_accounts_to_claim_remainder(accs.to_vec())
            .await?;
    }

    check_jars_after_migration(accounts, jar_after_rounding).await?;

    Ok(())
}

async fn check_jars_after_migration(users: Vec<AccountId>, contract: SweatJarContract<'_>) -> Result<()> {
    for user in users {
        for jar in contract.get_jars_for_account(user.clone()).await? {
            assert_eq!(jar.account_id, user);
            assert_eq!(jar.product_id, RegisterProductCommand::Locked12Months12Percents.id());
            assert_eq!(jar.principal.0, 100_000);
            assert_eq!(jar.claimed_balance.0, 0);
            assert_eq!(jar.is_penalty_applied, false);
        }
    }

    Ok(())
}
