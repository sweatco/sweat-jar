use std::{fs::read_to_string, time::Duration};

use anyhow::Result;
use near_workspaces::{network::Testnet, Account, Contract, Worker};
use nitka::{
    misc::ToNear,
    near_sdk::{serde_json, serde_json::Value},
};
use sweat_jar_model::{
    api::{ClaimApiIntegration, JarApiIntegration, ProductApiIntegration, SweatJarContract, WithdrawApiIntegration},
    claimed_amount_view::ClaimedAmountView,
    product::{FixedProductTermsDto, RegisterProductCommand, TermsDto, WithdrawalFeeDto},
    MS_IN_DAY, MS_IN_SECOND,
};
use sweat_model::SweatContract;
use tokio::time::sleep;

use crate::jar_contract_extensions::JarContractExtensions;

async fn acc_from_file(path: &str, worker: &Worker<Testnet>) -> Result<Account> {
    let account = Account::from_file(path, &worker)?;
    Ok(account)
}

async fn acc_with_name(name: &str, worker: &Worker<Testnet>) -> Result<Account> {
    let home = dirs::home_dir().unwrap();
    acc_from_file(
        &format!("{}/.near-credentials/testnet/{name}.json", home.display()),
        worker,
    )
    .await
}

async fn jar_testnet_contract(worker: &Worker<Testnet>) -> Result<Contract> {
    let account = acc_with_name("v8.jar.sweatty.testnet", worker).await?;
    let contract = Contract::from_secret_key(account.id().clone(), account.secret_key().clone(), worker);
    Ok(contract)
}

async fn token_testnet_contract(worker: &Worker<Testnet>) -> Result<Contract> {
    let account = acc_with_name("vfinal.token.sweat.testnet", worker).await?;
    let contract = Contract::from_secret_key(account.id().clone(), account.secret_key().clone(), worker);
    Ok(contract)
}

async fn testnet_user(worker: &Worker<Testnet>) -> Result<Account> {
    acc_with_name("testnet_user.testnet", worker).await
}

async fn testnet_user_2(worker: &Worker<Testnet>) -> Result<Account> {
    acc_with_name("testnet_user_2.testnet", worker).await
}

async fn jar_manager(worker: &Worker<Testnet>) -> Result<Account> {
    acc_with_name("bob_account.testnet", worker).await
}

fn _get_products() -> Vec<RegisterProductCommand> {
    let json_str = read_to_string("../products_testnet.json").unwrap();

    let json: Value = serde_json::from_str(&json_str).unwrap();

    let mut products: Vec<RegisterProductCommand> = vec![];

    for product_val in json.as_array().unwrap() {
        let id = product_val["product_id"].as_str().unwrap().to_string();

        let cap_min: u128 = product_val["min_amount"].as_str().unwrap().parse().unwrap();
        let cap_max: u128 = product_val["max_amount"].as_str().unwrap().parse().unwrap();

        let pk = product_val["public_key"].as_str().unwrap();

        #[allow(deprecated)]
        let pk = base64::decode(pk).unwrap();

        let is_enabled = product_val["is_enabled"].as_bool().unwrap();

        let withdrawal_fee = {
            let fixed: u128 = product_val["fee_fixed"].as_str().unwrap().parse().unwrap();
            let percent = product_val["fee_percent"].as_f64().unwrap();

            if fixed != 0 {
                Some(WithdrawalFeeDto::Fix(fixed.into()))
            } else if percent != 0.0 {
                Some(WithdrawalFeeDto::Percent(((percent * 1000.0) as u128).into(), 3))
            } else {
                None
            }
        };

        let apy = product_val["apy"].as_f64().unwrap();

        let lockup_seconds = product_val["lockup_seconds"].as_u64().unwrap();

        products.push(RegisterProductCommand {
            id,
            apy_default: (((apy * 1000.0) as u128).into(), 3),
            apy_fallback: None,
            cap_min: cap_min.into(),
            cap_max: cap_max.into(),
            terms: TermsDto::Fixed(FixedProductTermsDto {
                lockup_term: (lockup_seconds * MS_IN_SECOND).into(),
                allows_top_up: product_val["allows_top_up"].as_bool().unwrap(),
                allows_restaking: product_val["allows_restaking"].as_bool().unwrap(),
            }),
            withdrawal_fee,
            public_key: Some(pk.into()),
            is_enabled,
            score_cap: 0,
        })
    }

    products
}

async fn register_test_product(manager: &Account, jar: &SweatJarContract<'_>) -> Result<()> {
    jar.register_product(RegisterProductCommand {
        id: "5_days_20000_steps".to_string(),
        apy_default: (0.into(), 0),
        apy_fallback: None,
        cap_min: 1_000_000.into(),
        cap_max: 500000000000000000000000.into(),
        terms: TermsDto::Fixed(FixedProductTermsDto {
            lockup_term: (MS_IN_DAY * 5).into(),
            allows_top_up: false,
            allows_restaking: false,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
        score_cap: 20_000,
    })
    .with_user(manager)
    .await?;
    Ok(())
}

#[ignore]
#[tokio::test]
async fn register_product() -> Result<()> {
    let worker = near_workspaces::testnet().await?;

    let jar = jar_testnet_contract(&worker).await?;
    let jar = SweatJarContract { contract: &jar };

    register_test_product(&jar_manager(&worker).await?, &jar).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn create_many_jars() -> Result<()> {
    let worker = near_workspaces::testnet().await?;

    let user = testnet_user_2(&worker).await?;
    let _manager = jar_manager(&worker).await?;
    let token = token_testnet_contract(&worker).await?;
    let token = SweatContract { contract: &token };

    let jar = jar_testnet_contract(&worker).await?;
    let jar = SweatJarContract { contract: &jar };

    let jars = jar.get_jars_for_account(user.to_near()).await?;

    dbg!(&jars.len());

    for _ in 0..1000 {
        jar.create_jar(
            &user,
            "5min_50apy_restakable_no_signature".to_string(),
            1000000000000000000,
            &token,
        )
        .await?
        .0;
    }

    let jars = jar.get_jars_for_account(user.to_near()).await?;

    dbg!(&jars.len());

    Ok(())
}

#[ignore]
#[tokio::test]
/// Run this after testnet migration to check that everything runs as expected
async fn testnet_sanity_check() -> Result<()> {
    const PRODUCT_ID: &str = "testnet_migration_test_product";
    const PRINCIPAL: u128 = 1222333334567778000;

    let worker = near_workspaces::testnet().await?;

    let user = testnet_user(&worker).await?;
    let _manager = jar_manager(&worker).await?;
    let token = token_testnet_contract(&worker).await?;
    let token = SweatContract { contract: &token };

    let jar = jar_testnet_contract(&worker).await?;
    let jar = SweatJarContract { contract: &jar };

    let jars = jar.get_jars_for_account(user.to_near()).await?;

    jar.create_jar(&user, PRODUCT_ID.to_string(), PRINCIPAL, &token)
        .await?
        .0;

    let jars_after = jar.get_jars_for_account(user.to_near()).await?;

    assert_eq!(jars.len() + 1, jars_after.len());

    let new_jar = jars_after
        .into_iter()
        .filter(|item| !jars.contains(&item))
        .next()
        .unwrap();

    assert_eq!(new_jar.product_id, "testnet_migration_test_product");
    assert_eq!(new_jar.principal, PRINCIPAL.into());

    sleep(Duration::from_secs(5)).await;

    let withdrawn = jar.withdraw_all(None).with_user(&user).await?;

    assert!(withdrawn.jars.into_iter().any(|j| j.withdrawn_amount.0 == PRINCIPAL));

    let ClaimedAmountView::Detailed(claimed) = jar.claim_total(Some(true)).with_user(&user).await? else {
        panic!()
    };

    let claimed_jar = claimed.detailed.get(&new_jar.id).expect("New jar not found");

    assert_eq!(claimed_jar.0, 193799678869);

    let jars = jar.get_jars_for_account(user.to_near()).await?;

    // Jar is deleted after full claim and withdraw
    assert!(!jars.into_iter().any(|j| j.id == new_jar.id));

    Ok(())
}
