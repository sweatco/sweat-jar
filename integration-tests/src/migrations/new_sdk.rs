use anyhow::Result;
use integration_utils::misc::ToNear;
use near_sdk::json_types::U128;
use sweat_jar_model::api::{InitApiIntegration, JarApiIntegration, ProductApiIntegration, SweatJarContract};
use sweat_model::{FungibleTokenCoreIntegration, StorageManagementIntegration, SweatApiIntegration, SweatContract};

use crate::{
    jar_contract_extensions::JarContractExtensions, migrations::helpers::load_wasm, product::RegisterProductCommand,
};

#[tokio::test]
#[ignore]
async fn migrate_to_new_sdk() -> Result<()> {
    let ft_code = load_wasm("res/sweat.wasm");
    let jar_old_code = load_wasm("res/sweat_jar_main.wasm");
    let jar_new_code = load_wasm("res/sweat_jar.wasm");

    let worker = near_workspaces::sandbox().await?;

    let fee_account = worker.dev_create_account().await?;
    let manager_account = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    let ft_account = worker.dev_create_account().await?;
    let ft_contract = ft_account.deploy(&ft_code).await?.into_result()?;
    let ft_contract = SweatContract { contract: &ft_contract };

    ft_contract.new(".u.sweat.testnet".to_string().into()).await?;

    let jar_account = worker.dev_create_account().await?;
    let old_jar_contract = jar_account.deploy(&jar_old_code).await?.into_result()?;
    let old_jar_contract = SweatJarContract {
        contract: &old_jar_contract,
    };

    ft_contract.storage_deposit(jar_account.to_near().into(), None).await?;
    ft_contract.tge_mint(&jar_account.to_near(), U128(100_000_000)).await?;

    old_jar_contract
        .init(ft_account.to_near(), fee_account.to_near(), manager_account.to_near())
        .await?;

    ft_contract.tge_mint(&bob.to_near(), 1_000_000.into()).await?;

    old_jar_contract
        .register_product(RegisterProductCommand::Locked10Minutes6PercentsTopUp.get())
        .with_user(&manager_account)
        .await?;

    let products = old_jar_contract.get_products().with_user(&ft_account).await?;
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
    let new_jar_contract = SweatJarContract {
        contract: &new_jar_contract,
    };

    let products_new = new_jar_contract.get_products().with_user(&ft_account).await?;
    assert_eq!(products, products_new);

    let bob_jars_new = new_jar_contract.get_jars_for_account(bob.to_near()).await?;
    assert_eq!(bob_jars, bob_jars_new);

    new_jar_contract
        .register_product(RegisterProductCommand::Locked6Months6Percents.get())
        .with_user(&manager_account)
        .await?;

    let products = new_jar_contract.get_products().with_user(&ft_account).await?;
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
