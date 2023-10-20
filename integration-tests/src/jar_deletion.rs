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

    dbg!(&jar_view);

    context.fast_forward_minutes(11).await?;

    let alice_interest = context.jar_contract.get_total_interest(&alice).await?;
    dbg!(&alice_interest.get_interest());

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    dbg!(&jars);

    let withdrawn_amount = context.jar_contract.withdraw(&alice, jar_view.id).await?;
    dbg!(&withdrawn_amount);

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    dbg!(&jars);

    let claimed_amount = context.jar_contract.claim_total(&alice).await?;
    dbg!(&claimed_amount);

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    dbg!(&jars);

    let alice_interest = context.jar_contract.get_total_interest(&alice).await?;
    dbg!(&alice_interest.get_interest());

    // assert!(jars.is_empty());

    Ok(())
}
