#![cfg(test)]

use std::future::IntoFuture;

use near_workspaces::types::Gas;
use sweat_jar_model::api::ProductApiIntegration;

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
        context
            .sweat_jar()
            .register_product(command.get())
            .with_user(&manager)
            .into_future(),
    )
    .await?;

    Ok(gas)
}
