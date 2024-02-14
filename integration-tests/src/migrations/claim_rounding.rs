use anyhow::Result;
use fake::Fake;
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
    use std::time::Instant;
    let now = Instant::now();

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

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    let users_count = 5000;
    let jars_per_user_count = 2;
    let total_jars = users_count * jars_per_user_count;

    dbg!(total_jars);

    let mut accounts = Vec::with_capacity(users_count);

    for i in 0..users_count {
        accounts.push(AccountId::new_unchecked(64.fake::<String>().to_ascii_lowercase()));
    }

    let elapsed = now.elapsed();
    println!("Created users elapsed: {:.2?}", elapsed);

    for accs in accounts.chunks(850) {
        context
            .sweat_jar()
            .bulk_create_jars(
                accs.to_vec(),
                RegisterProductCommand::Locked12Months12Percents.id(),
                NearToken::from_near(30).as_yoctonear(),
                jars_per_user_count as u32,
            )
            .await?;
    }

    let elapsed = now.elapsed();
    println!("Created jars elapsed: {:.2?}", elapsed);

    let jar_after_rounding = load_wasm("res/sweat_jar.wasm");
    let jar_after_rounding = jar_account.deploy(&jar_after_rounding).await?.into_result()?;
    let jar_after_rounding = SweatJarContract {
        contract: &jar_after_rounding,
    };

    let elapsed = now.elapsed();
    println!("Updated contract elapsed: {:.2?}", elapsed);

    set_integration_logs_enabled(true);

    jar_after_rounding.migrate_state_to_claim_remainder().await?;

    for accs in accounts.chunks(600) {
        jar_after_rounding
            .migrate_accounts_to_claim_remainder(accs.to_vec())
            .await?;
    }

    let elapsed = now.elapsed();
    println!("Migrated elapsed: {:.2?}", elapsed);

    // check_jars_after_migration(accounts, jar_after_rounding).await?;
    //
    // let elapsed = now.elapsed();
    // println!("Check elapsed: {:.2?}", elapsed);

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
