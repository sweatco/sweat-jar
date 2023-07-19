use crate::context::Context;
use crate::product::Products;

pub(crate) async fn run() -> anyhow::Result<()> {
    let context = Context::new().await?;

    let manager = context.account("manager");
    let alice = context.account("alice");

    context.ft_contract.init().await?;
    context.jar_contract.init(context.ft_contract.account(), vec![context.account("manager").id()]).await?;

    context.ft_contract.storage_deposit(context.jar_contract.account()).await?;
    context.ft_contract.storage_deposit(&alice).await?;
    context.ft_contract.mint_for_user(&alice, 100_000_000).await?;

    context.jar_contract.register_product(&manager, Products::Locked12Months12Percents.json()).await?;
    context.jar_contract.register_product(&manager, Products::Locked6Months6Percents.json()).await?;
    context.jar_contract.register_product(&manager, Products::Locked6Months6PercentsWithWithdrawFee.json()).await?;

    let products = context.jar_contract.get_products().await?;
    assert_eq!(3, products.as_array().unwrap().len());

    context.jar_contract.create_jar(
        alice,
        Products::Locked12Months12Percents.id(),
        1_000_000,
        context.ft_contract.account().id(),
    ).await?;

    let mut alice_principal = context.jar_contract.get_total_principal(alice).await?;
    let mut alice_interest = context.jar_contract.get_total_interest(alice).await?;
    assert_eq!(1_000_000, alice_principal);
    assert_eq!(0, alice_interest);

    context.fast_forward(1).await?;

    alice_interest = context.jar_contract.get_total_interest(alice).await?;
    println!("@@ Alices interest = {:?}", alice_interest);

    Ok(())
}