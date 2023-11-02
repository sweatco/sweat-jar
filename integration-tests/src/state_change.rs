use std::{fs, path::PathBuf, process::Command};

use anyhow::Result;
use integration_utils::{integration_contract::IntegrationContract, misc::ToNear};
use model::api::{InitApiIntegration, JarApiIntegration, ProductApiIntegration};
use near_sdk::json_types::U128;
use sweat_integration::SweatFt;
use sweat_model::{FungibleTokenCoreIntegration, StorageManagementIntegration, SweatApiIntegration};

use crate::{jar_contract_interface::SweatJar, product::RegisterProductCommand};

fn load_wasm(wasm_path: &str) -> Vec<u8> {
    // Assuming that the Makefile is in root repository directory
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .unwrap();
    assert!(output.status.success(), "Failed to get Git repository root path");
    let git_root: PathBuf = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_string()
        .into();

    let wasm_filepath = fs::canonicalize(git_root.join(wasm_path)).expect("Failed to get wasm file path");
    fs::read(wasm_filepath).expect("Failed to load wasm")
}

#[tokio::test]
#[ignore]
async fn test_stage_change() -> Result<()> {
    let ft_code = load_wasm("res/sweat.wasm");
    let jar_old_code = load_wasm("res/sweat_jar_main.wasm");
    let jar_new_code = load_wasm("res/sweat_jar.wasm");

    let worker = near_workspaces::testnet().await?;

    let fee_account = worker.dev_create_account().await?;
    let manager_account = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    let ft_account = worker.dev_create_account().await?;
    let ft_contract = ft_account.deploy(&ft_code).await?.into_result()?;
    let mut ft_contract = SweatFt::with_contract(&ft_contract);

    ft_contract.new(".u.sweat.testnet".to_string().into()).await?;

    let jar_account = worker.dev_create_account().await?;
    let old_jar_contract = jar_account.deploy(&jar_old_code).await?.into_result()?;
    let mut old_jar_contract = SweatJar::with_contract(&old_jar_contract);

    ft_contract.storage_deposit(jar_account.to_near().into(), None).await?;
    ft_contract.tge_mint(&jar_account.to_near(), U128(100_000_000)).await?;

    old_jar_contract
        .init(ft_account.to_near(), fee_account.to_near(), manager_account.to_near())
        .await?;

    ft_contract.tge_mint(&bob.to_near(), 1_000_000.into()).await?;

    old_jar_contract
        .with_user(&manager_account)
        .register_product(RegisterProductCommand::Locked10Minutes6PercentsTopUp.get())
        .await?;

    let products = old_jar_contract.with_user(&ft_account).get_products().await?;
    assert_eq!(products.len(), 1);

    let bob_jars = old_jar_contract.get_jars_for_account(bob.to_near()).await?;
    assert!(bob_jars.is_empty());

    let staked = old_jar_contract
        .create_jar(
            &bob,
            RegisterProductCommand::Locked10Minutes6PercentsTopUp.get().id,
            100_000,
            ft_account.id(),
        )
        .await?;

    let bob_jars = old_jar_contract.get_jars_for_account(bob.to_near()).await?;

    assert_eq!(bob_jars.len(), 1);

    assert_eq!(staked.0, 100_000);

    assert_eq!(ft_contract.ft_balance_of(bob.to_near()).await?.0, 900_000);

    dbg!(ft_contract.ft_balance_of(bob.to_near()).await?);

    drop(old_jar_contract);

    let new_jar_contract = jar_account.deploy(&jar_new_code).await?.into_result()?;
    let mut new_jar_contract = SweatJar::with_contract(&new_jar_contract);

    let products_new = new_jar_contract.with_user(&ft_account).get_products().await?;
    assert_eq!(products, products_new);

    let bob_jars_new = new_jar_contract.get_jars_for_account(bob.to_near()).await?;
    assert_eq!(bob_jars, bob_jars_new);

    new_jar_contract
        .with_user(&manager_account)
        .register_product(RegisterProductCommand::Locked6Months6Percents.get())
        .await?;

    let products = new_jar_contract.with_user(&ft_account).get_products().await?;
    assert_eq!(products.len(), 2);

    let staked = new_jar_contract
        .create_jar(
            &bob,
            RegisterProductCommand::Locked10Minutes6PercentsTopUp.get().id,
            100_000,
            ft_account.id(),
        )
        .await?;

    let bob_jars = new_jar_contract.get_jars_for_account(bob.to_near()).await?;

    assert_eq!(bob_jars.len(), 2);

    assert_eq!(staked.0, 100_000);

    assert_eq!(ft_contract.ft_balance_of(bob.to_near()).await?.0, 800_000);

    Ok(())
}
