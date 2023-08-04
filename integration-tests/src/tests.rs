mod product;
mod ft_contract_interface;
mod jar_contract_interface;
mod context;
mod happy_flow;
mod migration;
mod withdraw_fee;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // withdraw_fee::run().await?;
    happy_flow::run().await?;
    // migration::run().await?;

    Ok(())
}
