#![cfg(test)]

use integration_utils::integration_contract::IntegrationContract;
use model::api::ProductApiIntegration;
use near_workspaces::types::Gas;

use crate::{
    context::{prepare_contract, IntegrationContext},
    measure::outcome_storage::OutcomeStorage,
    product::RegisterProductCommand,
};

#[mutants::skip]
pub(crate) async fn measure_register_product(command: RegisterProductCommand) -> anyhow::Result<Gas> {
    let mut context = prepare_contract([]).await?;

    let manager = context.manager().await?;

    let (gas, _) = OutcomeStorage::measure_operation(
        "register_product",
        &manager,
        context.sweat_jar().with_user(&manager).register_product(command.get()),
    )
    .await?;

    Ok(gas)
}
