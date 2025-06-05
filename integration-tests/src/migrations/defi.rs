use near_workspaces::types::NearToken;
use nitka::{misc::ToNear, near_sdk::serde_json::json};
use sweat_jar_model::api::{InitApiIntegration, JarApiIntegration, ProductApiIntegration};
use sweat_model::{FungibleTokenCoreIntegration, StorageManagementIntegration, SweatApiIntegration};

use crate::{
    context::{Context, IntegrationContext, FT_CONTRACT, SWEAT_JAR},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn defi_migration() -> anyhow::Result<()> {
    println!("👷🏽 Run migration test");

    let mut context = Context::new(&[FT_CONTRACT, SWEAT_JAR], true, "build-integration".into()).await?;

    let manager = &context.manager().await?;
    let alice = &context.alice().await?;
    let bob = &context.bob().await?;
    let fee_account = &context.fee().await?;
    let v2_account = &context.v2_account().await?;

    context.ft_contract().new(".u.sweat.testnet".to_string().into()).await?;
    context
        .sweat_jar()
        .init(
            context.ft_contract().contract.as_account().to_near(),
            fee_account.to_near(),
            manager.to_near(),
            v2_account.to_near(),
        )
        .await?;

    context
        .ft_contract()
        .storage_deposit(context.sweat_jar().contract.as_account().to_near().into(), None)
        .await?;

    context
        .ft_contract()
        .storage_deposit(manager.to_near().into(), None)
        .await?;
    context
        .ft_contract()
        .storage_deposit(alice.to_near().into(), None)
        .await?;
    context
        .ft_contract()
        .storage_deposit(bob.to_near().into(), None)
        .await?;

    context
        .ft_contract()
        .tge_mint(&manager.to_near(), 3_000_000.into())
        .await?;
    context
        .ft_contract()
        .tge_mint(&alice.to_near(), 100_000_000.into())
        .await?;
    context
        .ft_contract()
        .tge_mint(&bob.to_near(), 100_000_000_000.into())
        .await?;

    context
        .sweat_jar()
        .register_product(RegisterProductCommand::Locked12Months12Percents.get())
        .with_user(&manager)
        .await?;

    context.fast_forward_hours(1).await?;

    context
        .ft_contract()
        .ft_transfer_call(
            context.sweat_jar().contract.as_account().to_near(),
            3_000_000.into(),
            None,
            json!({
                "type": "migrate",
                "data": [
                    {
                        "id": "old_0",
                        "account_id": alice.id(),
                        "product_id": RegisterProductCommand::Locked12Months12Percents.id(),
                        "principal": "2000000",
                        "created_at": "0",
                    },
                    {
                        "id": "old_1",
                        "account_id": alice.id(),
                        "product_id": RegisterProductCommand::Locked12Months12Percents.id(),
                        "principal": "700000",
                        "created_at": "100",
                    },
                    {
                        "id": "old_2",
                        "account_id": bob.id(),
                        "product_id": RegisterProductCommand::Locked12Months12Percents.id(),
                        "principal": "300000",
                        "created_at": "0",
                    },
            ]
            })
            .to_string(),
        )
        .deposit(NearToken::from_yoctonear(1))
        .with_user(&manager)
        .await?;

    let manager_balance = context.ft_contract().ft_balance_of(manager.to_near()).await?;
    assert_eq!(0, manager_balance.0);

    let alice_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(2, alice_jars.len());

    let alice_first_jar = alice_jars.first().unwrap();
    assert_eq!(1, alice_first_jar.id.0);
    assert_eq!(2000000, alice_first_jar.principal.0);

    let alice_second_jar = alice_jars.get(1).unwrap();
    assert_eq!(2, alice_second_jar.id.0);
    assert_eq!(700000, alice_second_jar.principal.0);

    let alice_principal = context.sweat_jar().get_total_principal(alice.to_near()).await?;
    assert_eq!(2_700_000, alice_principal.total.0);

    let bob_principal = context.sweat_jar().get_total_principal(bob.to_near()).await?;
    assert_eq!(300_000, bob_principal.total.0);

    Ok(())
}
