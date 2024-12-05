use anyhow::Result;
use near_workspaces::{network::Sandbox, types::NearToken, Account, Worker};
use nitka::{build::build_contract, json, misc::ToNear, near_sdk::serde_json::Value};
use sweat_jar_model::{
    api::{InitApiIntegration, JarApiIntegration, ProductApiIntegration, StateMigrationIntegration, SweatJarContract},
    Timezone,
};
use sweat_model::{StorageManagementIntegration, SweatApiIntegration, SweatContract};

use crate::{
    jar_contract_extensions::JarContractExtensions, migrations::helpers::load_wasm, product::RegisterProductCommand,
};

#[tokio::test]
async fn migrate_state() -> Result<()> {
    build_contract("build-integration".into())?;

    let binaries = Binaries::load("res/sweat.wasm", "res/sweat_jar.wasm", "res/sweat_jar_3_4_0.wasm");
    let worker = near_workspaces::sandbox().await?;
    let accounts = Accounts::new(&worker).await?;

    let ft_contract = SweatContract {
        contract: &accounts.ft.deploy(&binaries.ft).await?.into_result()?,
    };
    let jar_contract = SweatJarContract {
        contract: &accounts.jar.deploy(&binaries.jar_legacy).await?.into_result()?,
    };

    ft_contract.new(".u.sweat.testnet".to_string().into()).await?;
    ft_contract.storage_deposit(accounts.jar.to_near().into(), None).await?;
    ft_contract
        .tge_mint(&accounts.jar.to_near(), 100_000_000.into())
        .await?;
    ft_contract
        .storage_deposit(accounts.alice.to_near().into(), None)
        .await?;
    ft_contract
        .tge_mint(&accounts.alice.to_near(), 1_000_000.into())
        .await?;

    // Before migration
    prepare_legacy_jar_contract(&accounts, &jar_contract, &ft_contract).await?;

    // Migration
    let jar_contract = SweatJarContract {
        contract: &accounts.jar.deploy(&binaries.jar).await?.into_result()?,
    };
    jar_contract.migrate_state().with_user(&accounts.manager).await?;

    // After migration
    let products = jar_contract.get_products().await?;
    assert_eq!(RegisterProductCommand::all().len(), products.len());

    let alice_jars = jar_contract.get_jars_for_account(accounts.alice.to_near()).await?;
    assert_eq!(3, alice_jars.len());

    Ok(())
}

async fn prepare_legacy_jar_contract<'a>(
    accounts: &Accounts,
    jar_contract: &SweatJarContract<'a>,
    ft_contract: &SweatContract<'a>,
) -> Result<()> {
    jar_contract
        .init(
            accounts.ft.to_near(),
            accounts.fee.to_near(),
            accounts.manager.to_near(),
        )
        .await?;

    for product in RegisterProductCommand::all() {
        let _result = accounts
            .manager
            .call(&jar_contract.contract.as_account().to_near(), "register_product")
            .args_json(json!({
                "command": product.json_legacy(),
            }))
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()?;
    }

    let products_json: Value = jar_contract.contract.call("get_products").view().await?.json()?;
    assert_eq!(
        RegisterProductCommand::all().len(),
        products_json.as_array().unwrap().len()
    );
    dbg!(products_json);

    let result = jar_contract
        .create_jar(
            &accounts.alice,
            RegisterProductCommand::Locked10Minutes6Percents.id(),
            1_000_00,
            ft_contract,
        )
        .result()
        .await?;
    dbg!(result);

    let result = jar_contract
        .create_jar(
            &accounts.alice,
            RegisterProductCommand::Flexible6Months6Percents.id(),
            1_000_00,
            ft_contract,
        )
        .result()
        .await?;
    dbg!(result);

    let result = jar_contract
        .create_step_jar(
            &accounts.alice,
            RegisterProductCommand::Locked10Minutes20000ScoreCap.id(),
            1_000_00,
            Timezone::hour_shift(0),
            ft_contract,
        )
        .result()
        .await?;
    dbg!(result);

    let alice_jars = jar_contract.get_jars_for_account(accounts.alice.to_near()).await?;
    assert_eq!(3, alice_jars.len());

    Ok(())
}

struct Binaries {
    ft: Vec<u8>,
    jar: Vec<u8>,
    jar_legacy: Vec<u8>,
}

impl Binaries {
    fn load(ft_path: &str, jar_path: &str, jar_legacy_path: &str) -> Self {
        Binaries {
            ft: load_wasm(ft_path),
            jar: load_wasm(jar_path),
            jar_legacy: load_wasm(jar_legacy_path),
        }
    }
}

struct Accounts {
    ft: Account,
    jar: Account,
    manager: Account,
    fee: Account,
    alice: Account,
}

impl Accounts {
    async fn new(worker: &Worker<Sandbox>) -> Result<Self> {
        let instance = Accounts {
            ft: worker.dev_create_account().await?,
            jar: worker.dev_create_account().await?,
            manager: worker.dev_create_account().await?,
            fee: worker.dev_create_account().await?,
            alice: worker.dev_create_account().await?,
        };

        Ok(instance)
    }
}
