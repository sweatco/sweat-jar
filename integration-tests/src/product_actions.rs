use near_sdk::json_types::Base64VecU8;

use crate::{
    common::{prepare_contract, Prepared, ValueGetters},
    product::RegisterProductCommand,
};

#[tokio::test]
async fn happy_flow() -> anyhow::Result<()> {
    println!("👷🏽 Run happy flow test");

    let Prepared {
        context,
        manager,
        alice,
        fee_account: _,
    } = prepare_contract([
        RegisterProductCommand::Locked12Months12Percents,
        RegisterProductCommand::Locked6Months6Percents,
        RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
    ])
    .await?;

    context
        .jar_contract
        .set_enabled(&manager, RegisterProductCommand::Locked12Months12Percents.id(), false)
        .await?;

    context
        .jar_contract
        .set_public_key(
            &manager,
            RegisterProductCommand::Locked12Months12Percents.id(),
            "".to_string(), // TODO: set real pk
        )
        .await?;

    Ok(())
}
