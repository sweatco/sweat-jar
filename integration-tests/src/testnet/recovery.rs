use std::{fs::read_to_string, time::Duration};

use anyhow::Result;
use near_workspaces::Account;
use nitka::{
    misc::ToNear,
    near_sdk::{serde_json, serde_json::Value},
};
use sweat_jar_model::{
    api::{ClaimApiIntegration, JarApiIntegration, ProductApiIntegration, SweatJarContract, WithdrawApiIntegration},
    claimed_amount_view::ClaimedAmountView,
    product::{Apy, Cap, FixedProductTerms, Product, ScoreBasedProductTerms, Terms, WithdrawalFee},
    UDecimal, MS_IN_DAY, MS_IN_SECOND,
};
use sweat_model::FungibleTokenCoreIntegration;
use tokio::time::sleep;

use crate::{jar_contract_extensions::JarContractExtensions, testnet::testnet_context::TestnetContext};

fn _get_products() -> Vec<Product> {
    let json_str = read_to_string("../products_testnet.json").unwrap();

    let json: Value = serde_json::from_str(&json_str).unwrap();

    let mut products: Vec<Product> = vec![];

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
                Some(WithdrawalFee::Fix(fixed.into()))
            } else if percent != 0.0 {
                Some(WithdrawalFee::Percent(UDecimal::new((percent * 1000.0) as u128, 3)))
            } else {
                None
            }
        };

        let apy = product_val["apy"].as_f64().unwrap();

        let lockup_seconds = product_val["lockup_seconds"].as_u64().unwrap();

        products.push(Product {
            id,
            cap: Cap::new(cap_min, cap_max),
            terms: Terms::Fixed(FixedProductTerms {
                lockup_term: (lockup_seconds * MS_IN_SECOND).into(),
                apy: Apy::Constant(UDecimal::new((apy * 1000.0) as u128, 3)),
            }),
            withdrawal_fee,
            public_key: Some(pk.into()),
            is_enabled,
            is_restakable: true,
        })
    }

    products
}

async fn register_test_product(manager: &Account, jar: &SweatJarContract<'_>) -> Result<()> {
    jar.register_product(Product {
        id: "5_days_20000_score".to_string(),
        cap: Cap::new(1_000_000, 500_000_000_000_000_000_000_000),
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: (MS_IN_DAY * 5).into(),
            score_cap: 20_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
        is_restakable: true,
    })
    .with_user(manager)
    .await?;
    Ok(())
}

#[ignore]
#[tokio::test]
async fn register_product() -> Result<()> {
    let ctx = TestnetContext::new().await?;

    register_test_product(&ctx.manager, &ctx.jar_contract()).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn create_many_jars() -> Result<()> {
    let ctx = TestnetContext::new().await?;

    let jars = ctx.jar_contract().get_jars_for_account(ctx.user.to_near()).await?;

    dbg!(&jars.len());

    for _ in 0..1000 {
        ctx.jar_contract()
            .create_jar(
                &ctx.user,
                "5min_50apy_restakable_no_signature".to_string(),
                1000000000000000000,
                &ctx.token_contract(),
            )
            .await?
            .0;
    }

    let jars = ctx.jar_contract().get_jars_for_account(ctx.user.to_near()).await?;

    dbg!(&jars.len());

    Ok(())
}

#[ignore]
#[tokio::test]
/// Run this after testnet migration to check that everything runs as expected
async fn testnet_sanity_check() -> Result<()> {
    const PRODUCT_ID: &str = "testnet_migration_test_product";
    const PRINCIPAL: u128 = 1222333334567778000;

    let ctx = TestnetContext::new().await?;

    let jars = ctx.jar_contract().get_jars_for_account(ctx.user.to_near()).await?;

    ctx.jar_contract()
        .create_jar(&ctx.user, PRODUCT_ID.to_string(), PRINCIPAL, &ctx.token_contract())
        .await?
        .0;

    let jars_after = ctx.jar_contract().get_jars_for_account(ctx.user.to_near()).await?;

    assert_eq!(jars.len() + 1, jars_after.len());

    let new_jar = jars_after
        .into_iter()
        .filter(|item| !jars.contains(&item))
        .next()
        .unwrap();

    assert_eq!(new_jar.product_id, "testnet_migration_test_product");
    assert_eq!(new_jar.principal, PRINCIPAL.into());

    sleep(Duration::from_secs(5)).await;

    let user_balance_before_withdrawal = ctx.token_contract().ft_balance_of(ctx.user.to_near()).await?;
    ctx.jar_contract().withdraw_all().with_user(&ctx.user).await?;
    let user_balance_after_withdrawal = ctx.token_contract().ft_balance_of(ctx.user.to_near()).await?;

    assert_eq!(
        PRINCIPAL,
        user_balance_after_withdrawal.0 - user_balance_before_withdrawal.0
    );

    let ClaimedAmountView::Detailed(claimed) = ctx.jar_contract().claim_total(Some(true)).with_user(&ctx.user).await?
    else {
        panic!()
    };

    let claimed_jar = claimed.detailed.get(&new_jar.id).expect("New jar not found");

    assert_eq!(claimed_jar.0, 193799678869);

    let jars = ctx.jar_contract().get_jars_for_account(ctx.user.to_near()).await?;

    // Jar is deleted after full claim and withdraw
    assert!(!jars.into_iter().any(|j| j.id == new_jar.id));

    Ok(())
}

#[ignore]
#[tokio::test]
async fn sandbox() -> Result<()> {
    let ctx = TestnetContext::new().await?;

    let jars = ctx.jar_contract().get_jars_for_account(ctx.user2.to_near()).await?;
    dbg!(&jars);

    ctx.jar_contract()
        .unlock_jars_for_account(ctx.user2.to_near())
        .with_user(&ctx.manager)
        .await?;

    let jars = ctx.jar_contract().get_jars_for_account(ctx.user2.to_near()).await?;
    dbg!(&jars);

    Ok(())
}
