use near_workspaces::types::Gas;
use sweat_jar_model::api::ProductApiIntegration;

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand,
};

#[mutants::skip]
pub(crate) async fn measure_register_product(command: RegisterProductCommand) -> anyhow::Result<Gas> {
    let mut context = prepare_contract(None, []).await?;

    let manager = context.manager().await?;

    Ok(context
        .sweat_jar()
        .register_product(command.get())
        .with_user(&manager)
        .result()
        .await?
        .total_gas_burnt)
}
