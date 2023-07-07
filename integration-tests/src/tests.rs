use near_units::parse_near;
use serde_json::json;
use std::collections::HashMap;
use std::{env, fs};
use workspaces::{Account, Contract, AccountId};

struct Context {
    pub accounts: HashMap<String, Account>,
    pub contract: Contract,
    pub token_contract: Contract,
}

impl Context {
    async fn new() -> anyhow::Result<Context> {
        println!("Initializing context");

        let worker = workspaces::sandbox().await?;
        let account = worker.dev_create_account().await?;

        let mut accounts = HashMap::<String, Account>::new();

        let manager = account
            .create_subaccount("manager")
            .initial_balance(parse_near!("3 N"))
            .transact()
            .await?
            .into_result()?;
        accounts.insert("manager".to_string(), manager);

        let alice = account
            .create_subaccount("alice")
            .initial_balance(parse_near!("3 N"))
            .transact()
            .await?
            .into_result()?;
        accounts.insert("alice".to_string(), alice);

        let contract = worker
            .dev_deploy(&Self::load_wasm(&(env::args().nth(1).unwrap())))
            .await?;
        let token_contract = worker
            .dev_deploy(&Self::load_wasm(&(env::args().nth(2).unwrap())))
            .await?;

        Result::Ok(Context {
            contract,
            token_contract,
            accounts,
        })
    }

    fn account(&self, name: &str) -> &Account {
        self.accounts.get(name).expect("Account doesn't exist")
    }

    fn load_wasm(wasm_path: &str) -> Vec<u8> {
        let current_dir = env::current_dir().expect("Failed to get current dir");
        let wasm_filepath =
            fs::canonicalize(current_dir.join(wasm_path)).expect("Failed to get wasm file path");
        std::fs::read(wasm_filepath).expect("Failed to load wasm")
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let context = Context::new().await?;
    let contract = context.contract.clone();
    let token_contract = context.token_contract.clone();

    let manager = context.account("manager");
    let alice = context.account("alice");

    init_token_contract(&token_contract).await?;
    init_jar_contract(&contract, &token_contract.as_account(), vec![context.account("manager").id()]).await?;

    register_in_ft(&contract.as_account(), &token_contract).await?;
    register_in_ft(&alice, &token_contract).await?;

    register_product(&manager, &contract).await?;
    get_products(&alice, &contract).await?;

    Ok(())
}

async fn init_token_contract(contract: &Contract) -> anyhow::Result<()> {
    println!("▶️ Init ft contract");

    contract
        .call("new")
        .args_json(json!({
            "postfix": ".u.sweat.testnet",
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

async fn init_jar_contract(contract: &Contract, token_contract_account: &Account, admin_allowlist: Vec<&AccountId>) -> anyhow::Result<()> {
    println!("▶️ Init jar contract");

    contract
        .call("init")
        .args_json(json!({
            "token_account_id": token_contract_account.id(),
            "admin_allowlist": admin_allowlist,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

async fn register_in_ft(user: &Account, contract: &Contract) -> anyhow::Result<()> {
    println!("▶️ Register {} in ft contract", user.id());

    let args = json!({
        "account_id": user.id()
    });

    user.call(contract.id(), "storage_deposit")
        .args_json(args)
        .deposit(parse_near!("0.00235 N"))
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

async fn register_product(user: &Account, contract: &Contract) -> anyhow::Result<()> {
    let args = json!({
        "product": {
            "id": "regular_10",
            "lockup_term": 1_314_000_000_000_u64,
            "maturity_term": 1_314_000_000_000_u64,
            "is_refillable": false,
            "apy": {
                "Constant": {
                    "significand": 1,
                    "exponent": 1,
                },
            },
            "cap": 100,
            "is_restakable": false,
        },
    });

    user.call(contract.id(), "register_product")
        .args_json(args)
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

async fn get_products(user: &Account, contract: &Contract) -> anyhow::Result<()> {
    let products: serde_json::Value = user
        .call(contract.id(), "get_products")
        .view()
        .await?
        .json()?;

    println!("@@ Products = {:?}", products);

    Ok(())
}
