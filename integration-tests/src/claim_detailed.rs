use crate::{
    common::{prepare_contract, Prepared, ValueGetters},
    product::RegisterProductCommand,
};

#[tokio::test]
async fn claim_detailed() -> anyhow::Result<()> {
    println!("👷🏽 Run detailed claim test");

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
    let alice_interest = context.jar_contract.get_total_interest(&alice).await?;
    assert_eq!(1_000_000, alice_principal.get_u128("total"));
    assert_eq!(0, alice_interest.get_interest());

    context.fast_forward_hours(1).await?;

    let claimed_details = context.jar_contract.claim_total_detailed(&alice).await?;
    let claimed_amount = claimed_details.total.0;

    assert!(15 < claimed_amount && claimed_amount < 20);
    assert_eq!(
        claimed_amount,
        claimed_details.detailed.values().map(|item| item.0).sum()
    );

    Ok(())
}
