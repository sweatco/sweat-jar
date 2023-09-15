use workspaces::{types::Gas, Account};

use crate::{
    common::{prepare_contract, Prepared},
    context::Context,
    measure::outcome_storage::OutcomeStorage,
    product::RegisterProductCommand,
};

pub(crate) async fn measure_after_claim_total(jars_count: usize) -> anyhow::Result<Gas> {
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

    for _ in 0..jars_count {
        add_jar(
            &context,
            &alice,
            RegisterProductCommand::Locked12Months12Percents,
            100_000,
        )
        .await?;
    }

    context.fast_forward(1).await?;

    let gas = OutcomeStorage::measure("interest_to_claim", &alice, context.jar_contract.claim_total(&alice)).await?;

    Ok(gas)
}

async fn add_jar(
    context: &Context,
    account: &Account,
    product: RegisterProductCommand,
    amount: u128,
) -> anyhow::Result<()> {
    context
        .jar_contract
        .create_jar(account, product.id(), amount, context.ft_contract.account().id())
        .await?;

    Ok(())
}
