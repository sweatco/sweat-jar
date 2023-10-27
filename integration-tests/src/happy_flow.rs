use crate::{
    common::{prepare_contract, Prepared, ValueGetters},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn happy_flow() -> anyhow::Result<()> {
    println!("👷🏽 Run happy flow test");

    let Prepared {
        context,
        manager: _,
        alice,
        fee_account: _,
    } = prepare_contract([
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ])
    .await?;

    let coverage = context.jar_contract.get_coverage().await?;

    std::fs::write("output.profraw", coverage).unwrap();

    let products = context.jar_contract.get_products().await?;
    assert_eq!(3, products.as_array().unwrap().len());

    context
        .jar_contract
        .create_jar(
            &alice,
            RegisterProductCommand::Locked12Months12Percents.id(),
            1_000_000,
            context.ft_contract.account().id(),
        )
        .await?;

    let alice_principal = context.jar_contract.get_total_principal(&alice).await?;
    let mut alice_interest = context.jar_contract.get_total_interest(&alice).await?;
    assert_eq!(1_000_000, alice_principal.get_u128("total"));
    assert_eq!(0, alice_interest.get_interest());

    context.fast_forward_hours(1).await?;

    alice_interest = context.jar_contract.get_total_interest(&alice).await?;
    assert!(alice_interest.get_interest() > 0);

    let claimed_amount = context.jar_contract.claim_total(&alice).await?;
    assert!(15 < claimed_amount && claimed_amount < 20);

    let alice_balance = context.ft_contract.ft_balance_of(&alice).await?.0;
    assert_eq!(99_000_000 + claimed_amount, alice_balance);

    Ok(())
}
