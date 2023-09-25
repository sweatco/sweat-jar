use near_sdk::json_types::Base64VecU8;

use crate::{
    common::{prepare_contract, Prepared, ValueGetters},
    product,
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
    } = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;

    let product_id = RegisterProductCommand::Locked12Months12Percents.id();

    let result = context
        .jar_contract
        .create_jar(
            &alice,
            product_id.clone(),
            1_000_000,
            context.ft_contract.account().id(),
        )
        .await?;

    println!("1. Result of first jar creation: {:?}", result);

    context
        .jar_contract
        .set_enabled(&manager, RegisterProductCommand::Locked12Months12Percents.id(), false)
        .await?;

    let result = context
        .jar_contract
        .create_jar(&alice, product_id, 100, context.ft_contract.account().id())
        .await?;

    println!("2. Result of second jar creation: {:?}", result);

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
