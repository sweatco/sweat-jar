#![cfg(test)]

use near_workspaces::types::Gas;

use crate::{
    common::{prepare_contract, Prepared},
    measure::outcome_storage::OutcomeStorage,
    product::RegisterProductCommand,
};

#[mutants::skip]
pub(crate) async fn measure_register_product(command: RegisterProductCommand) -> anyhow::Result<Gas> {
    let Prepared {
        context,
        manager,
        alice: _,
        fee_account: _,
    } = prepare_contract([]).await?;

    let (gas, _) = OutcomeStorage::measure_operation(
        "register_product",
        &manager,
        context.jar_contract.register_product(&manager, command.json()),
    )
    .await?;

    Ok(gas)
}
