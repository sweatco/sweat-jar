use fake::Fake;
use rand::{prelude::IteratorRandom, thread_rng};
use workspaces::Account;

use crate::{
    common::{prepare_contract, Prepared},
    context::Context,
    product::RegisterProductCommand,
};

async fn add_random_jar(
    context: &Context,
    account: &Account,
    products: &[RegisterProductCommand],
) -> anyhow::Result<()> {
    context
        .jar_contract
        .create_jar(
            account,
            products.iter().choose(&mut thread_rng()).unwrap().id(),
            (100_000..500_000).fake(),
            context.ft_contract.account().id(),
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn many_jars() -> anyhow::Result<()> {
    println!("👷🏽 Run many jars flow test");

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

    for _ in 0..10
    //(10..15).fake()
    {
        add_random_jar(
            &context,
            &alice,
            &[
                RegisterProductCommand::Locked12Months12Percents,
                // &RegisterProductCommand::Locked6Months6Percents.id(),
                // &RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee.id(),
            ],
        )
        .await?;
    }

    context.fast_forward(1).await?;

    let claimed_amount = context.jar_contract.claim_total(&alice).await?;

    dbg!(&claimed_amount);

    //assert!(15 < claimed_amount && claimed_amount < 20);

    // let alice_balance = context.ft_contract.ft_balance_of(alice).await?.0;
    // assert_eq!(99_000_000 + claimed_amount, alice_balance);

    Ok(())
}
