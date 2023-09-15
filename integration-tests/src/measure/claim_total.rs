use std::sync::Mutex;

use workspaces::{types::Gas, Account};

use crate::{
    common::{prepare_contract, Prepared},
    context::Context,
    measure::outcome_storage::OutcomeStorage,
    product::RegisterProductCommand,
};

static CONTRACT: Mutex<RegisterProductCommand> = Mutex::new(RegisterProductCommand::Locked6Months6Percents);

pub(crate) fn set_claim_total_contract(contract: RegisterProductCommand) {
    *CONTRACT.lock().unwrap() = contract;
}

fn get_contract() -> RegisterProductCommand {
    *CONTRACT.lock().unwrap()
}

#[ignore]
#[tokio::test]
async fn one_after_claim() -> anyhow::Result<()> {
    set_claim_total_contract(RegisterProductCommand::Locked6Months6Percents);

    let gas = measure_after_claim_total(1).await?;

    dbg!(&gas);

    Ok(())
}

pub(crate) async fn measure_after_claim_total(jars_count: usize) -> anyhow::Result<Gas> {
    let Prepared {
        context,
        manager: _,
        alice,
        fee_account: _,
    } = prepare_contract([get_contract()]).await?;

    for _ in 0..jars_count {
        dbg!("add jar");
        add_jar(&context, &alice, get_contract(), 100_000).await?;
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
