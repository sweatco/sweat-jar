use anyhow::Result;
use near_workspaces::{network::Testnet, Account, Contract, Worker};
use sweat_jar_model::api::SweatJarContract;
use sweat_model::SweatContract;

pub struct TestnetContext {
    token_contract: Contract,

    pub manager: Account,
    pub user: Account,
    #[allow(dead_code)]
    pub user2: Account,

    jar_contract: Contract,
}

impl TestnetContext {
    pub async fn new() -> Result<Self> {
        let worker = near_workspaces::testnet().await?;

        let user = testnet_user(&worker).await?;
        let user2 = testnet_user_2(&worker).await?;
        let manager = jar_manager(&worker).await?;
        let token_contract = token_testnet_contract(&worker).await?;

        let jar_contract = jar_testnet_contract(&worker).await?;

        Ok(Self {
            token_contract,
            manager,
            user,
            user2,
            jar_contract,
        })
    }

    pub fn jar_contract(&self) -> SweatJarContract<'_> {
        SweatJarContract {
            contract: &self.jar_contract,
        }
    }

    pub fn token_contract(&self) -> SweatContract<'_> {
        SweatContract {
            contract: &self.token_contract,
        }
    }
}

async fn acc_from_file(path: &str, worker: &Worker<Testnet>) -> Result<Account> {
    let account = Account::from_file(path, &worker)?;
    Ok(account)
}

async fn acc_with_name(name: &str, worker: &Worker<Testnet>) -> Result<Account> {
    let home = dirs::home_dir().unwrap();
    acc_from_file(&format!("{}/.near-credentials/testnet/{name}", home.display()), worker).await
}

async fn jar_testnet_contract(worker: &Worker<Testnet>) -> Result<Contract> {
    let account = acc_with_name("v8.jar.sweatty.testnet.json", worker).await?;
    let contract = Contract::from_secret_key(account.id().clone(), account.secret_key().clone(), worker);
    Ok(contract)
}

async fn token_testnet_contract(worker: &Worker<Testnet>) -> Result<Contract> {
    let account = acc_with_name("vfinal.token.sweat.testnet.json", worker).await?;
    let contract = Contract::from_secret_key(account.id().clone(), account.secret_key().clone(), worker);
    Ok(contract)
}

async fn testnet_user(worker: &Worker<Testnet>) -> Result<Account> {
    acc_with_name("testnet_user.testnet.json", worker).await
}

async fn testnet_user_2(worker: &Worker<Testnet>) -> Result<Account> {
    acc_with_name("testnet_user_3.testnet.json", worker).await
}

async fn jar_manager(worker: &Worker<Testnet>) -> Result<Account> {
    acc_with_name("bob_account.testnet.json", worker).await
}
