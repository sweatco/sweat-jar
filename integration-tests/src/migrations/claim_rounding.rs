use anyhow::Result;
use fake::Fake;
use near_workspaces::types::NearToken;
use nitka::{contract_call::set_integration_logs_enabled, misc::ToNear};
use sweat_jar_model::{
    api::{
        IntegrationTestMethodsIntegration, JarApiIntegration, MigrationToClaimRemainderIntegration, SweatJarContract,
    },
    jar::JarView,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    migrations::helpers::load_wasm,
    product::RegisterProductCommand,
};

#[tokio::test]
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

    let alice = context.alice().await?;

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    let users_count = 5000;
    let jars_per_user_count = 2;
    let total_jars = users_count * jars_per_user_count;

    dbg!(total_jars);

    let mut accounts = Vec::with_capacity(users_count);

    for _ in 0..users_count {
        accounts.push(64.fake::<String>().to_ascii_lowercase().try_into().unwrap());
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

    const PRINCIPAL: u128 = 100000;

    context
        .sweat_jar()
        .create_jar(
            &alice,
            RegisterProductCommand::Locked6Months6Percents.id(),
            PRINCIPAL,
            &context.ft_contract(),
        )
        .await?;

    let alice_jars_before = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    let alice_principal = context.sweat_jar().get_total_principal(alice.to_near()).await?;

    assert_eq!(alice_principal.total.0, PRINCIPAL);

    let jar_after_rounding = load_wasm("res/sweat_jar.wasm");
    let jar_after_rounding = jar_account.deploy(&jar_after_rounding).await?.into_result()?;
    let jar_after_rounding = SweatJarContract {
        contract: &jar_after_rounding,
    };

    let elapsed = now.elapsed();
    println!("Updated contract elapsed: {:.2?}", elapsed);

    set_integration_logs_enabled(true);

    jar_after_rounding.migrate_state_to_claim_remainder().await?;

    let alice_jars_after = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    let alice_principal_after = context.sweat_jar().get_total_principal(alice.to_near()).await?;

    assert_eq!(alice_jars_before, alice_jars_after);
    assert_eq!(alice_principal, alice_principal_after);
    assert_eq!(alice_principal_after.total.0, PRINCIPAL);

    context
        .sweat_jar()
        .create_jar(
            &alice,
            RegisterProductCommand::Locked6Months6Percents.id(),
            PRINCIPAL,
            &context.ft_contract(),
        )
        .await?;

    let alice_principal_2_jars = context.sweat_jar().get_total_principal(alice.to_near()).await?;
    assert_eq!(alice_principal_2_jars.total.0, PRINCIPAL * 2);

    let alice_2_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;

    let jar = alice_jars_before.into_iter().next().unwrap();

    assert_eq!(
        alice_2_jars.clone(),
        vec![
            jar.clone(),
            JarView {
                id: alice_2_jars[1].id,
                created_at: alice_2_jars[1].created_at,
                ..jar
            },
        ]
    );

    for accs in accounts.chunks(600) {
        jar_after_rounding
            .migrate_accounts_to_claim_remainder(accs.to_vec())
            .await?;
    }

    let elapsed = now.elapsed();
    println!("Migrated elapsed: {:.2?}", elapsed);

    Ok(())
}
