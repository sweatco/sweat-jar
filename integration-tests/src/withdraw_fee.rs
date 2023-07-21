use crate::context::Context;
use crate::product::Products;

pub(crate) async fn run() -> anyhow::Result<()> {
    let mut context = Context::new().await?;

    let manager = &context.account("manager").await?;
    let alice = &context.account("alice").await?;
    let fee_account = &context.account("fee").await?;

    context.ft_contract.init().await?;
    context.jar_contract.init(context.ft_contract.account(), fee_account, vec![manager.id()]).await?;

    context.ft_contract.storage_deposit(context.jar_contract.account()).await?;
    context.ft_contract.storage_deposit(fee_account).await?;
    context.ft_contract.storage_deposit(alice).await?;
    context.ft_contract.mint_for_user(alice, 100_000_000).await?;

    context.jar_contract.register_product(manager, Products::Locked10Minutes6PercentsWithWithdrawFee.json()).await?;

    context.jar_contract.create_jar(
        alice,
        Products::Locked10Minutes6PercentsWithWithdrawFee.id(),
        1_000_000,
        context.ft_contract.account().id(),
    ).await?;

    let mut alice_balance = context.ft_contract.ft_balance_of(alice).await?;
    assert_eq!(99_000_000, alice_balance.0);

    context.fast_forward(1).await?;

    context.jar_contract.withdraw(alice, "0".to_string()).await?;

    alice_balance = context.ft_contract.ft_balance_of(alice).await?;
    assert_eq!(99_999_000, alice_balance.0);

    let fee_balance = context.ft_contract.ft_balance_of(fee_account).await?;
    assert_eq!(1_000, fee_balance.0);

    Ok(())
}