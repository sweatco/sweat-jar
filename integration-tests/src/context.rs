use std::{collections::HashMap, env, fs};

use near_units::parse_near;
use workspaces::{network::Sandbox, Account, Worker};

use crate::{ft_contract_interface::FtContractInterface, jar_contract_interface::JarContractInterface};

const EPOCH_BLOCKS_HEIGHT: u64 = 43_200;
const HOURS_PER_EPOCH: u64 = 12;
const ONE_HOUR_BLOCKS_HEIGHT: u64 = EPOCH_BLOCKS_HEIGHT / HOURS_PER_EPOCH;

pub(crate) struct Context {
    worker: Worker<Sandbox>,
    root_account: Account,
    pub accounts: HashMap<String, Account>,
    pub ft_contract: Box<dyn FtContractInterface>,
    pub jar_contract: Box<dyn JarContractInterface>,
}

impl Context {
    pub(crate) async fn new() -> anyhow::Result<Context> {
        println!("🏭 Initializing context");

        let worker = workspaces::sandbox().await?;
        let root_account = worker.dev_create_account().await?;

        let jar_contract_path = env::args().nth(1).unwrap_or("../res/sweat_jar.wasm".to_string());
        let ft_contract_path = env::args().nth(2).unwrap_or("../res/sweat.wasm".to_string());

        let jar_contract = worker.dev_deploy(&Self::load_wasm(&jar_contract_path)).await?;
        let ft_contract = worker.dev_deploy(&Self::load_wasm(&ft_contract_path)).await?;

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

    pub(crate) async fn fast_forward(&self, hours: u64) -> anyhow::Result<()> {
        let blocks_to_advance = ONE_HOUR_BLOCKS_HEIGHT * hours;

        println!("⏳ Fast forward to {hours} hours ({blocks_to_advance} blocks)...");

        self.worker.fast_forward(blocks_to_advance).await?;

        Ok(())
    }
}
