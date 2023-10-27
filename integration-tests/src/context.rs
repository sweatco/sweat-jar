use std::{collections::HashMap, env, fs};

use near_units::parse_near;
use near_workspaces::{network::Sandbox, Account, Worker};

use crate::{
    common::build_contract, ft_contract_interface::FtContractInterface, jar_contract_interface::JarContractInterface,
};

const ONE_MINUTE_BLOCKS_HEIGHT: u64 = 240;

pub(crate) struct Context {
    worker: Worker<Sandbox>,
    root_account: Account,
    pub accounts: HashMap<String, Account>,
    pub ft_contract: Box<dyn FtContractInterface + Send + Sync>,
    pub jar_contract: Box<dyn JarContractInterface + Send + Sync>,
}

impl Context {
    pub(crate) async fn new() -> anyhow::Result<Context> {
        println!("🏭 Initializing context");

        build_contract()?;

        let worker = near_workspaces::sandbox().await?;
        let root_account = worker.dev_create_account().await?;

        let jar_contract = worker.dev_deploy(&Self::load_wasm("../res/sweat_jar.wasm")).await?;
        let ft_contract = worker.dev_deploy(&Self::load_wasm("../res/sweat.wasm")).await?;

        println!("@@ jar contract deployed to {}", jar_contract.id());
        println!("@@ ft contract deployed to {}", ft_contract.id());

        Ok(Context {
            worker,
            root_account,
            accounts: HashMap::new(),
            ft_contract: Box::new(ft_contract),
            jar_contract: Box::new(jar_contract),
        })
    }

    pub(crate) async fn account(&mut self, name: &str) -> anyhow::Result<Account> {
        if !self.accounts.contains_key(name) {
            let account = self
                .root_account
                .create_subaccount(name)
                .initial_balance(parse_near!("3 N"))
                .transact()
                .await?
                .into_result()?;

            self.accounts.insert(name.to_string(), account);
        }

        Ok(self.accounts.get(name).unwrap().clone())
    }

    fn load_wasm(wasm_path: &str) -> Vec<u8> {
        let current_dir = env::current_dir().expect("Failed to get current dir");
        let wasm_filepath = fs::canonicalize(current_dir.join(wasm_path)).expect("Failed to get wasm file path");
        fs::read(wasm_filepath).expect("Failed to load wasm")
    }

    pub(crate) async fn fast_forward_hours(&self, hours: u64) -> anyhow::Result<()> {
        self.fast_forward_minutes(hours * 60).await
    }

    pub(crate) async fn fast_forward_minutes(&self, minutes: u64) -> anyhow::Result<()> {
        let blocks_to_advance = ONE_MINUTE_BLOCKS_HEIGHT * minutes;
        println!("⏳ Fast forward to {minutes} minutes ({blocks_to_advance} blocks)...");
        self.worker.fast_forward(blocks_to_advance).await?;
        Ok(())
    }

    pub(crate) fn get_signature_material(
        &self,
        receiver_id: &Account,
        product_id: &String,
        valid_until: u64,
        amount: u128,
        last_jar_id: Option<String>,
    ) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.jar_contract.account().id(),
            receiver_id.id(),
            product_id,
            amount,
            last_jar_id.map_or_else(String::new, |value| value,),
            valid_until,
        )
    }
}
