use fake::Fake;
use rand::{prelude::IteratorRandom, thread_rng};
use workspaces::Account;

use crate::{context::Context, product::RegisterProductCommand};

async fn add_random_jar(context: &Context, account: &Account, products: &[&str]) -> anyhow::Result<()> {
    context
        .jar_contract
        .create_jar(
            account,
            products.iter().choose(&mut thread_rng()).unwrap().to_string(),
            (100_000..500_000).fake(),
            context.ft_contract.account().id(),
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn many_jars() -> anyhow::Result<()> {
    return Ok(());

    println!("👷🏽 Run many jars flow test");

    let mut context = Context::new().await?;

    let manager = &context.account("manager").await?;
    let alice = &context.account("alice").await?;

    context.ft_contract.init().await?;
    context
        .jar_contract
        .init(context.ft_contract.account(), manager, manager.id())
        .await?;

    context
        .ft_contract
        .storage_deposit(context.jar_contract.account())
        .await?;
    context.ft_contract.storage_deposit(alice).await?;
    context.ft_contract.mint_for_user(alice, 100_000_000).await?;

    context
        .jar_contract
        .register_product(manager, RegisterProductCommand::Locked12Months12Percents.json())
        .await?;
    context
        .jar_contract
        .register_product(manager, RegisterProductCommand::Locked6Months6Percents.json())
        .await?;
    context
        .jar_contract
        .register_product(
            manager,
            RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee.json(),
        )
        .await?;

    for _ in 0..10
    //(10..15).fake()
    {
        add_random_jar(
            &context,
            alice,
            &[
                &RegisterProductCommand::Locked12Months12Percents.id(),
                // &RegisterProductCommand::Locked6Months6Percents.id(),
                // &RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee.id(),
            ],
        )
        .await?;
    }

    let alice_principal = context.jar_contract.get_total_principal(alice).await?;

    dbg!(&alice_principal);

    let mut alice_interest = context.jar_contract.get_total_interest(alice).await?;

    dbg!(&alice_interest);

    // assert_eq!(1_000_000, alice_principal.get_u128("total"));
    // assert_eq!(0, alice_interest.get_u128("total"));

    context.fast_forward(1).await?;
    dbg!("Fast forward");

    alice_interest = context.jar_contract.get_total_interest(alice).await?;

    dbg!(&alice_interest);

    // assert!(alice_interest.get_u128("total") > 0);

    let claimed_amount = context.jar_contract.claim_total(alice).await?;

    dbg!(&claimed_amount);

    //assert!(15 < claimed_amount && claimed_amount < 20);

    // let alice_balance = context.ft_contract.ft_balance_of(alice).await?.0;
    // assert_eq!(99_000_000 + claimed_amount, alice_balance);

    Ok(())
}
