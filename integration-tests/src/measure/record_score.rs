use anyhow::Result;
use fake::Fake;
use near_workspaces::Account;
use nitka::{
    measure::utils::pretty_gas_string, misc::ToNear, near_sdk::json_types::U128, set_integration_logs_enabled,
};
use sweat_jar_model::{
    api::{IntegrationTestMethodsIntegration, ScoreApiIntegration},
    Timezone, MS_IN_HOUR,
};
use sweat_model::{StorageManagementIntegration, SweatApiIntegration};

use crate::{
    context::{prepare_contract, Context, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_record_score_single_account() -> Result<()> {
    set_integration_logs_enabled(false);

    let mut ctx = prepare_contract(None, [RegisterProductCommand::Locked10Minutes20000ScoreCap]).await?;
    let admin = ctx.manager().await?;

    let user = prepare_account_with_step_jars(&mut ctx, 1600).await?;

    let now = ctx.sweat_jar().block_timestamp_ms().await?;

    let gas = ctx
        .sweat_jar()
        .record_score(vec![(user.to_near(), vec![(10_000, (now - MS_IN_HOUR).into())])])
        .with_user(&admin)
        .result()
        .await?
        .total_gas_burnt;

    dbg!(pretty_gas_string(gas));

    //    1  jar on account -   6 TGas 196 GGas total:   6196465753523
    //  100 jars on account -  23 TGas 807 GGas total:  23807581190747
    // 1000 jars on account - 182 TGas  56 GGas total: 182056233612011
    // 1600 jars on account - 290 TGas 927 GGas total: 290927837363347

    Ok(())
}

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_record_score_multiple_accounts() -> Result<()> {
    const NUMBER_OF_ACCOUNTS: usize = 100;
    const NUMBER_OF_JARS: u16 = 20;

    set_integration_logs_enabled(false);

    let mut ctx = prepare_contract(None, [RegisterProductCommand::Locked10Minutes20000ScoreCap]).await?;
    let admin = ctx.manager().await?;

    let mut accounts = vec![];

    for _ in 0..NUMBER_OF_ACCOUNTS {
        accounts.push(prepare_account_with_step_jars(&mut ctx, NUMBER_OF_JARS).await?);
    }

    let now = ctx.sweat_jar().block_timestamp_ms().await?;

    let mut records = vec![];

    for account in accounts {
        records.push((account.to_near(), vec![(10_000, (now - MS_IN_HOUR).into())]));
    }

    loop {
        let result = ctx
            .sweat_jar()
            .record_score(records.clone())
            .with_user(&admin)
            .result()
            .await;

        if result.is_err() {
            if records.is_empty() {
                dbg!("NOPE");
                break;
            }

            dbg!("Failed, trying fewer accounts");

            records.pop();
            dbg!(records.len());
            continue;
        }

        let gas = result?.total_gas_burnt;

        dbg!(pretty_gas_string(gas));

        dbg!(records.len());

        break;
    }

    //  72 accounts each has  20 jars - 299 TGas
    //  31 accounts each has  50 jars - 299 TGas
    //  16 accounts each has 100 jars - 298 TGas
    //   8 accounts each has 200 jars - 294 TGas
    //   3 accounts each has 500 jars - 273 TGas

    Ok(())
}

async fn prepare_account_with_step_jars(ctx: &mut Context, jars: u16) -> Result<Account> {
    let account = ctx.account(&54.fake::<String>().to_lowercase()).await?;
    let fee = ctx.fee().await?;
    let admin = ctx.manager().await?;

    ctx.ft_contract().storage_deposit(fee.to_near().into(), None).await?;

    ctx.ft_contract()
        .tge_mint(&account.to_near(), U128(100_000_000_000_000_000))
        .await?;

    ctx.sweat_jar()
        .create_step_jar(
            &account,
            RegisterProductCommand::Locked10Minutes20000ScoreCap.id(),
            100_000,
            Timezone::hour_shift(0),
            &ctx.ft_contract(),
        )
        .await?;

    ctx.sweat_jar()
        .bulk_create_jars(
            account.to_near(),
            RegisterProductCommand::Locked10Minutes20000ScoreCap.id(),
            10_000,
            jars,
        )
        .with_user(&admin)
        .await?;

    Ok(account)
}
