#![cfg(test)]

use std::sync::Mutex;

use workspaces::{types::Gas, Account};

use crate::{
    common::{prepare_contract, Prepared},
    context::Context,
    measure::{measure::scoped_command_measure, outcome_storage::OutcomeStorage},
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
async fn measure_after_claim_total_test() -> anyhow::Result<()> {
    let mut results = vec![];

    for product in [
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ] {
        set_claim_total_contract(product);

        let measure_after_claim_total = scoped_command_measure(1..10, measure_after_claim_total).await?;

        let mut difs = vec![];

        for i in (1..measure_after_claim_total.len()).rev() {
            let diff = measure_after_claim_total[i].1 - measure_after_claim_total[i - 1].1;
            difs.push(diff);
        }

        results.push((product, measure_after_claim_total, difs))
    }

    dbg!(results);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn one_after_claim() -> anyhow::Result<()> {
    set_claim_total_contract(RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee);

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

    context.fast_forward_hours(2).await?;

    let (gas, _claimed) =
        OutcomeStorage::measure("interest_to_claim", &alice, context.jar_contract.claim_total(&alice)).await?;

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
