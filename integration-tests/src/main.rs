mod common;
mod context;
mod ft_contract_interface;
mod happy_flow;
mod jar_contract_interface;
mod migration;
mod product;
mod withdraw_fee;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    happy_flow::run().await?;
    withdraw_fee::run().await?;
    migration::run().await?;

    Ok(())
}
