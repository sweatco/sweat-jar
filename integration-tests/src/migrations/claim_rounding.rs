use anyhow::Result;
use fake::Fake;
use integration_utils::{contract_call::set_integration_logs_enabled, misc::ToNear};
use near_sdk::AccountId;
use near_workspaces::{network::Testnet, types::NearToken, Account, Contract, Worker};
use sweat_jar_model::{
    api::{
        IntegrationTestMethodsIntegration, JarApiIntegration, MigrationToClaimRemainderIntegration,
        ProductApiIntegration, SweatJarContract,
    },
    jar::JarView,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    migrations::helpers::load_wasm,
    product::RegisterProductCommand,
};

async fn acc_from_file(path: &str, worker: &Worker<Testnet>) -> Result<Account> {
    #[allow(deprecated)]
    let home = std::env::home_dir().unwrap();
    let path = format!("{}/.near-credentials/testnet/{path}", home.to_string_lossy());
    let account = Account::from_file(path, &worker)?;
    Ok(account)
}

async fn testnet_jar_contract(worker: &Worker<Testnet>) -> Result<Account> {
    acc_from_file("v8.jar.sweatty.testnet.json", worker).await
}

async fn acc_1(worker: &Worker<Testnet>) -> Result<Account> {
    acc_from_file("sweat_testnet_1.json", worker).await
}

async fn acc_2(worker: &Worker<Testnet>) -> Result<Account> {
    acc_from_file("sweat_testnet_2.json", worker).await
}

fn updated_code() -> Vec<u8> {
    #[allow(deprecated)]
    load_wasm(&format!(
        "{}/sweat-jar/res/sweat_jar.wasm",
        std::env::home_dir().unwrap().to_string_lossy()
    ))
}

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

    let alice = context.alice().await?;

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    let users_count = 5000;
    let jars_per_user_count = 2;
    let total_jars = users_count * jars_per_user_count;

    dbg!(total_jars);

    let mut accounts = Vec::with_capacity(users_count);

    for _ in 0..users_count {
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

    const PRINCIPAL: u128 = 100000;

    context
        .sweat_jar()
        .create_jar(
            &alice,
            RegisterProductCommand::Locked6Months6Percents.id(),
            PRINCIPAL,
            context.ft_contract().contract.as_account().id(),
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
            context.ft_contract().contract.as_account().id(),
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

#[tokio::test]
#[ignore]
async fn check_roundings_migration() -> Result<()> {
    let worker = near_workspaces::testnet().await?;

    let jar_account = testnet_jar_contract(&worker).await?;

    let jar_contract = Contract::from_secret_key(jar_account.id().clone(), jar_account.secret_key().clone(), &worker);
    let jar_contract = SweatJarContract {
        contract: &jar_contract,
    };

    let auto_migrate_acc = acc_1(&worker).await?;
    let manual_migrate_acc = acc_2(&worker).await?;

    assert_total_principal(&jar_contract, &manual_migrate_acc, 1511568730000000000000000).await?;
    assert_total_principal(&jar_contract, &auto_migrate_acc, 1979104990000000000000000).await?;

    let _updated_code = updated_code();

    let contract = jar_account.deploy(&_updated_code).await?.into_result()?;
    let jar_contract = SweatJarContract { contract: &contract };

    jar_contract
        .migrate_accounts_to_claim_remainder(vec![manual_migrate_acc.to_near()])
        .await?;

    assert_total_principal(&jar_contract, &manual_migrate_acc, 1511568730000000000000000).await?;
    assert_total_principal(&jar_contract, &auto_migrate_acc, 1979104990000000000000000).await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn check_jar_creation_on_testnet() -> Result<()> {
    let worker = near_workspaces::testnet().await?;

    let jar_account = testnet_jar_contract(&worker).await?;

    let jar_contract = Contract::from_secret_key(jar_account.id().clone(), jar_account.secret_key().clone(), &worker);
    let jar_contract = SweatJarContract {
        contract: &jar_contract,
    };

    let bob = acc_1(&worker).await?;

    let jars = jar_contract.get_jars_for_account(bob.to_near()).await?;
    dbg!(&jars.len());

    let principal = jar_contract.get_total_principal(bob.to_near()).await?.total.0;
    dbg!(&principal);

    let products = jar_contract.get_products().await?;

    dbg!(&products);

    let token: near_workspaces::AccountId = "vfinal.token.sweat.testnet".to_string().try_into().unwrap();

    jar_contract
        .create_jar(&bob, "locked_1_day_100_percents".into(), 1000000000000000000, &token)
        .await?;

    let jars = jar_contract.get_jars_for_account(bob.to_near()).await?;
    dbg!(&jars.len());

    let principal = jar_contract.get_total_principal(bob.to_near()).await?.total.0;
    dbg!(&principal);

    //
    // "locked_1_day_100_percents"

    Ok(())
}

async fn assert_total_principal(
    contract: &SweatJarContract<'_>,
    account: &Account,
    expected_principal: u128,
) -> Result<()> {
    let jars = contract.get_jars_for_account(account.to_near()).await?;
    let account_principal: u128 = jars.into_iter().map(|j| j.principal.0).sum();
    assert_eq!(account_principal, expected_principal);
    Ok(())
}
