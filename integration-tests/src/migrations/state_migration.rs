use anyhow::Result;
use near_workspaces::{network::Sandbox, types::NearToken, Account, Contract, Worker};
use nitka::{build::build_contract, json, misc::ToNear, near_sdk::serde_json::Value};
use sweat_jar_model::{
    api::{InitApiIntegration, JarApiIntegration, ProductApiIntegration, StateMigrationIntegration, SweatJarContract},
    Timezone, TokenAmount,
};
use sweat_model::{StorageManagementIntegration, SweatApiIntegration, SweatContract};

use crate::{
    jar_contract_extensions::{JarContractExtensions, JarContractLegacyExtensions},
    migrations::helpers::load_wasm,
    product::RegisterProductCommand,
};

#[tokio::test]
async fn migrate_state() -> Result<()> {
    build_contract("build-integration".into())?;

    let binaries = Binaries::load("res/sweat.wasm", "res/sweat_jar.wasm", "res/sweat_jar_3_4_0.wasm");
    let worker = near_workspaces::sandbox().await?;
    let accounts = Accounts::new(&worker).await?;

    let mut client = Client::init(&accounts, &binaries).await?;
    client.prepare(&accounts).await?;

    // Before migration
    let mut alice_interest = 0;
    prepare_legacy_jar_contract(&accounts, &client, &mut alice_interest).await?;

    // Migration
    client.migrate_jar_contract(&binaries, &accounts).await?;

    // After migration
    check_jar_contract_after_migration(accounts, &client, &alice_interest).await?;

    Ok(())
}

async fn prepare_legacy_jar_contract(
    accounts: &Accounts,
    client: &Client,
    alice_interest: &mut TokenAmount,
) -> Result<()> {
    client
        .jar()
        .init(
            accounts.ft.to_near(),
            accounts.fee.to_near(),
            accounts.manager.to_near(),
        )
        .await?;

    for product in RegisterProductCommand::all() {
        let _result = accounts
            .manager
            .call(&client.jar().contract.as_account().to_near(), "register_product")
            .args_json(json!({
                "command": product.json_legacy(),
            }))
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()?;
    }

    let products_json: Value = client.jar().contract.call("get_products").view().await?.json()?;
    assert_eq!(
        RegisterProductCommand::all().len(),
        products_json.as_array().unwrap().len()
    );

    client
        .jar()
        .create_jar(
            &accounts.alice,
            RegisterProductCommand::Locked10Minutes6Percents.id(),
            100000000000000000000000000,
            &client.ft(),
        )
        .result()
        .await?;
    client
        .jar()
        .create_jar(
            &accounts.alice,
            RegisterProductCommand::Flexible6Months6Percents.id(),
            100000000000000000000000000,
            &client.ft(),
        )
        .result()
        .await?;
    client
        .jar()
        .create_legacy_step_jar(
            &accounts.alice,
            RegisterProductCommand::Locked10Minutes20000ScoreCap.id(),
            100000000000000000000000000,
            Timezone::hour_shift(0),
            &client.ft(),
        )
        .result()
        .await?;

    let alice_jars = client.jar().get_jars_for_account(accounts.alice.to_near()).await?;
    assert_eq!(3, alice_jars.len());

    let result = client.jar().get_total_interest(accounts.alice.to_near()).await?;
    *alice_interest = result.amount.total.0;

    Ok(())
}

async fn check_jar_contract_after_migration(
    accounts: Accounts,
    client: &Client,
    alice_interest: &TokenAmount,
) -> Result<()> {
    let products = client.jar().get_products().await?;
    assert_eq!(RegisterProductCommand::all().len(), products.len());

    let alice_jars = client.jar().get_jars_for_account(accounts.alice.to_near()).await?;
    assert_eq!(3, alice_jars.len());

    let current_alice_interest = client.jar().get_total_interest(accounts.alice.to_near()).await?;
    assert!(current_alice_interest.amount.total.0 > *alice_interest);

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

struct Contracts {
    ft: Contract,
    jar: Contract,
}

impl Contracts {
    async fn new_legacy(accounts: &Accounts, binaries: &Binaries) -> Result<Self> {
        Ok(Self {
            ft: accounts.ft.deploy(&binaries.ft).await?.into_result()?,
            jar: accounts.jar.deploy(&binaries.jar_legacy).await?.into_result()?,
        })
    }

    async fn new_migrated(accounts: &Accounts, binaries: &Binaries) -> Result<Self> {
        Ok(Self {
            ft: accounts.ft.deploy(&binaries.ft).await?.into_result()?,
            jar: accounts.jar.deploy(&binaries.jar).await?.into_result()?,
        })
    }
}

struct Client {
    contracts: Contracts,
}

impl Client {
    async fn init(accounts: &Accounts, binaries: &Binaries) -> Result<Self> {
        Ok(Self {
            contracts: Contracts::new_legacy(accounts, binaries).await?,
        })
    }

    pub fn ft(&self) -> SweatContract<'_> {
        SweatContract {
            contract: &self.contracts.ft,
        }
    }

    pub fn jar(&self) -> SweatJarContract<'_> {
        SweatJarContract {
            contract: &self.contracts.jar,
        }
    }

    async fn prepare(&self, accounts: &Accounts) -> Result<()> {
        self.ft().new(".u.sweat.testnet".to_string().into()).await?;
        self.ft().storage_deposit(accounts.jar.to_near().into(), None).await?;
        self.ft()
            .tge_mint(&accounts.jar.to_near(), 10000000000000000000000000000.into())
            .await?;
        self.ft().storage_deposit(accounts.alice.to_near().into(), None).await?;
        self.ft()
            .tge_mint(&accounts.alice.to_near(), 10000000000000000000000000000.into())
            .await?;

        Ok(())
    }

    async fn migrate_jar_contract(&mut self, binaries: &Binaries, accounts: &Accounts) -> Result<()> {
        self.contracts = Contracts::new_migrated(accounts, binaries).await?;
        self.jar().migrate_state().with_user(&accounts.manager).await?;

        Ok(())
    }
}
