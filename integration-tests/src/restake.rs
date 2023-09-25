use crate::{
    common::{prepare_contract, Prepared, ValueGetters},
    product::RegisterProductCommand,
};

#[tokio::test]
async fn restake() -> anyhow::Result<()> {
    println!("👷🏽 Run test for restaking");

    let product_command = RegisterProductCommand::Locked10Minutes6Percents;
    let product_id = product_command.id();

    let Prepared {
        context,
        manager: _,
        alice,
        fee_account: _,
    } = prepare_contract([product_command]).await?;

    let amount = 1_000_000;
    context
        .jar_contract
        .create_jar(&alice, product_id, amount, context.ft_contract.account().id())
        .await?;

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    let original_jar_id = jars.as_array().unwrap().get(0).unwrap().get_jar_id();

    context.fast_forward_hours(1).await?;

    context.jar_contract.restake(&alice, original_jar_id.clone()).await?;

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    let jars_array = jars.as_array().unwrap();
    assert_eq!(jars_array.len(), 2);

    let mut has_original_jar = false;
    let mut has_restaked_jar = false;
    for jar in jars_array {
        let id = jar.get_jar_id();

        if id == original_jar_id {
            has_original_jar = true;
            assert_eq!(jar.get("principal").unwrap().as_str().unwrap(), "0");
        } else {
            has_restaked_jar = true;
            assert_eq!(jar.get("principal").unwrap().as_str().unwrap(), amount.to_string());
        }
    }

    assert!(has_original_jar);
    assert!(has_restaked_jar);

    context
        .jar_contract
        .claim_jars(&alice, vec![original_jar_id], None)
        .await?;

    let jars = context.jar_contract.get_jars_for_account(&alice).await?;
    let jars_array = jars.as_array().unwrap();
    assert_eq!(jars_array.len(), 1);

    Ok(())
}
