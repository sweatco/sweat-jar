use crate::context::Context;
use crate::product::RegisterProductCommand;

pub(crate) async fn run() -> anyhow::Result<()> {
    println!("👷🏽 Run happy flow test");

    let mut context = Context::new().await?;

    let manager = &context.account("manager").await?;
    let alice = &context.account("alice").await?;

    context.ft_contract.init().await?;
    context.jar_contract.init(context.ft_contract.account(), manager, manager.id()).await?;

    context.ft_contract.storage_deposit(context.jar_contract.account()).await?;
    context.ft_contract.storage_deposit(alice).await?;
    context.ft_contract.mint_for_user(alice, 100_000_000).await?;

    context.jar_contract.register_product(manager, RegisterProductCommand::Locked12Months12Percents.json()).await?;
    context.jar_contract.register_product(manager, RegisterProductCommand::Locked6Months6Percents.json()).await?;
    context.jar_contract.register_product(manager, RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee.json()).await?;

    let products = context.jar_contract.get_products().await?;
    assert_eq!(3, products.as_array().unwrap().len());

    context.jar_contract.create_jar(
        alice,
        RegisterProductCommand::Locked12Months12Percents.id(),
        1_000_000,
        context.ft_contract.account().id(),
    ).await?;

    let alice_principal = context.jar_contract.get_total_principal(alice).await?;
    let mut alice_interest = context.jar_contract.get_total_interest(alice).await?;
    assert_eq!(1_000_000, alice_principal.0);
    assert_eq!(0, alice_interest.0);

    context.fast_forward(1).await?;

    alice_interest = context.jar_contract.get_total_interest(alice).await?;
    assert!(alice_interest.0 > 0);

    let claimed_amount = context.jar_contract.claim_total(alice).await?;
    assert!(15 < claimed_amount && claimed_amount < 20);

    let alice_balance = context.ft_contract.ft_balance_of(alice).await?.0;
    assert_eq!(99_000_000 + claimed_amount, alice_balance);

    Ok(())
}