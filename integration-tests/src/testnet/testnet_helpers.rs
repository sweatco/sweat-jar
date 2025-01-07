use anyhow::Result;
use near_workspaces::{network::Testnet, Account, Contract, Worker};

pub(super) async fn acc_from_file(path: &str, worker: &Worker<Testnet>) -> Result<Account> {
    let account = Account::from_file(path, &worker)?;
    Ok(account)
}

pub(super) async fn acc_with_name(name: &str, worker: &Worker<Testnet>) -> Result<Account> {
    let home = dirs::home_dir().unwrap();
    acc_from_file(
        &format!("{}/.near-credentials/testnet/{name}.json", home.display()),
        worker,
    )
    .await
}

pub(super) async fn jar_testnet_contract(worker: &Worker<Testnet>, name: &str) -> Result<Contract> {
    let account = acc_with_name(name, worker).await?;
    let contract = Contract::from_secret_key(account.id().clone(), account.secret_key().clone(), worker);
    Ok(contract)
}

pub(super) async fn token_testnet_contract(worker: &Worker<Testnet>) -> Result<Contract> {
    let account = acc_with_name("vfinal.token.sweat.testnet", worker).await?;
    let contract = Contract::from_secret_key(account.id().clone(), account.secret_key().clone(), worker);
    Ok(contract)
}
