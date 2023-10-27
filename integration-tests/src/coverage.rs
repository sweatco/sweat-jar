use crate::{
    common::{prepare_contract, Prepared},
    product::RegisterProductCommand,
};

#[tokio::test]
async fn coverage() -> anyhow::Result<()> {
    println!("👷🏽 Run happy flow test");

    let Prepared {
        context,
        manager: _,
        alice: _,
        fee_account: _,
    } = prepare_contract([RegisterProductCommand::Locked12Months12Percents]).await?;

    let coverage: &[u8] = &context.jar_contract.get_coverage().await?;
    let coverage: Vec<u8> = serde_json::from_slice(coverage).unwrap();

    println!(
        "Values {} {} {} {} {} {} {}",
        coverage[0], coverage[1], coverage[2], coverage[3], coverage[4], coverage[5], coverage[6]
    );

    std::fs::write("output.profraw", coverage).unwrap();

    Ok(())
}
