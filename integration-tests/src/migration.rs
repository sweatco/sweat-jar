use serde_json::json;
use crate::context::Context;
use crate::product::Products;

pub(crate) async fn run() -> anyhow::Result<()> {
    let context = Context::new().await?;

    let manager = context.account("manager");
    let alice = context.account("alice");
    let bob = context.account("bob");

    context.ft_contract.init().await?;
    context.jar_contract.init(context.ft_contract.account(), vec![manager.id()]).await?;

    context.ft_contract.storage_deposit(context.jar_contract.account()).await?;
    context.ft_contract.storage_deposit(manager).await?;
    context.ft_contract.storage_deposit(alice).await?;
    context.ft_contract.storage_deposit(bob).await?;

    context.ft_contract.mint_for_user(manager, 3_000_000).await?;
    context.ft_contract.mint_for_user(alice, 100_000_000).await?;
    context.ft_contract.mint_for_user(bob, 100_000_000_000).await?;

    context.jar_contract.register_product(manager, Products::Locked12Months12Percents.json()).await?;

    context.fast_forward(1).await?;

    context.ft_contract.ft_transfer_call(
        manager,
        context.jar_contract.account().id(),
        3_000_000,
        json!({
            "action": "migrate",
            "data": [
                {
                    "id": "old_0",
                    "account_id": alice.id(),
                    "product_id": Products::Locked12Months12Percents.id(),
                    "principal": "2000000",
                    "created_at": 0,
                    "claimed_amount": "7",
                    "last_claim_at": 900_000,
                },
                {
                    "id": "old_1",
                    "account_id": alice.id(),
                    "product_id": Products::Locked12Months12Percents.id(),
                    "principal": "700000",
                    "created_at": 100,
                    "claimed_amount": "0",
                    "last_claim_at": null,
                },
                {
                    "id": "old_2",
                    "account_id": bob.id(),
                    "product_id": Products::Locked12Months12Percents.id(),
                    "principal": "300000",
                    "created_at": 0,
                    "claimed_amount": "0",
                    "last_claim_at": null,
                },
        ]
        }).to_string(),
    ).await?;

    let manager_balance = context.ft_contract.ft_balance_of(manager).await?;
    assert_eq!(0, manager_balance.0);

    let alice_jars_result = context.jar_contract.get_jars_for_account(alice).await?;
    let alice_jars = alice_jars_result.as_array().unwrap();
    assert_eq!(2, alice_jars.len());

    let alice_first_jar = alice_jars.get(0).unwrap();
    assert_eq!(0, alice_first_jar.get("index").unwrap().as_u64().unwrap());
    assert_eq!("2000000", alice_first_jar.get("principal").unwrap().as_str().unwrap());

    let alice_second_jar = alice_jars.get(1).unwrap();
    assert_eq!(1, alice_second_jar.get("index").unwrap().as_u64().unwrap());
    assert_eq!("700000", alice_second_jar.get("principal").unwrap().as_str().unwrap());

    let alice_principal = context.jar_contract.get_total_principal(alice).await?;
    assert_eq!(2_700_000, alice_principal);

    let bob_principal = context.jar_contract.get_total_principal(bob).await?;
    assert_eq!(300_000, bob_principal);

    Ok(())
}