use anyhow::Result;
use nitka::misc::{load_wasm, ToNear};
use sweat_jar_model::{
    api::{InitApiIntegration, JarApiIntegration, ProductApiIntegration, SweatJarContract},
    product::RegisterProductCommand,
    Timezone,
};
use sweat_model::{StorageManagementIntegration, SweatApiIntegration, SweatContract};

use crate::{
    jar_contract_extensions::JarContractExtensions,
    testnet::{
        testnet_context::TestnetContext,
        testnet_helpers::{acc_with_name, token_testnet_contract},
    },
};

#[ignore]
#[tokio::test]
async fn prepare_score_load_user() -> Result<()> {
    let ctx = TestnetContext::new().await?;

    ctx.jar_contract()
        .create_step_jar(
            &ctx.user,
            "step_jar_55_000".into(),
            100_000_000,
            Timezone::hour_shift(0),
            &ctx.token_contract(),
        )
        .await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn create_score_load_product() -> Result<()> {
    let ctx = TestnetContext::new().await?;

    ctx.jar_contract()
        .register_product(RegisterProductCommand {
            id: "step_jar_55_000".to_string(),
            apy_default: (Default::default(), 0),
            apy_fallback: None,
            cap_min: 10.into(),
            cap_max: u128::MAX.into(),
            terms: Default::default(),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
            score_cap: 55_000,
        })
        .with_user(&ctx.manager)
        .await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn deploy_and_prepare_contract() -> Result<()> {
    let worker = near_workspaces::testnet().await?;

    let user = acc_with_name("user_load_oracle_testing.testnet", &worker).await?;

    let jar_account = acc_with_name("jar_contract_load_oracle_testing.testnet", &worker).await?;

    let jar_contract = jar_account
        .deploy(&load_wasm("../res/sweat_jar.wasm"))
        .await?
        .into_result()?;

    let jar_contract = SweatJarContract {
        contract: &jar_contract,
    };

    let token_contract = token_testnet_contract(&worker).await?;

    let token_contract = SweatContract {
        contract: &token_contract,
    };

    let manager = acc_with_name("bob_account.testnet", &worker).await?;

    jar_contract
        .init(
            token_contract.contract.as_account().to_near(),
            manager.to_near(),
            manager.to_near(),
        )
        .await?;

    token_contract
        .storage_deposit(jar_contract.contract.as_account().to_near().into(), None)
        .await?;

    token_contract
        .tge_mint(&jar_contract.contract.as_account().to_near(), 100_000_000.into())
        .await?;

    token_contract.storage_deposit(user.to_near().into(), None).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn misc() -> Result<()> {
    let ctx = TestnetContext::new().await?;

    // ctx.token_contract()
    //     .tge_mint(&ctx.user.to_near(), 100_000_000_000.into())
    //     .await?;

    dbg!(ctx.jar_contract().get_jars_for_account(ctx.user.to_near()).await?);

    Ok(())
}
