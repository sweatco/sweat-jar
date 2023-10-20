use crate::{
    common::{prepare_contract, Prepared, ValueGetters},
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn jar_deletion() -> anyhow::Result<()> {
    println!("👷🏽 Run jar deletion test");

    let Prepared {
        context,
        manager: _,
        alice,
        fee_account: _,
    } = prepare_contract([RegisterProductCommand::Locked10Minutes60000Percents]).await?;

    context
        .jar_contract
        .create_jar(
            &alice,
            RegisterProductCommand::Locked10Minutes60000Percents.id(),
            1_000_000,
            context.ft_contract.account().id(),
        )
        .await?;

    let jar_view = context
        .jar_contract
        .get_jars_for_account(&alice)
        .await?
        .into_iter()
        .next()
        .unwrap();

    context.fast_forward_minutes(11).await?;

    let withdrawn_amount = context.jar_contract.withdraw(&alice, jar_view.id).await?;
    assert_eq!(withdrawn_amount.withdrawn_amount.0, 1_000_000);

    let alice_interest = context.jar_contract.get_total_interest(&alice).await?.get_interest();
    let claimed_amount = context.jar_contract.claim_total(&alice).await?;
    assert_eq!(alice_interest, claimed_amount);

    let alice_interest = context.jar_contract.get_total_interest(&alice).await?;
    assert_eq!(alice_interest.get_interest(), 0);

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    assert!(jars.is_empty());

    Ok(())
}
