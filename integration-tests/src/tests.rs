mod product;
mod ft_contract_interface;
mod jar_contract_interface;
mod context;
mod happy_flow;
mod migration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    happy_flow::run().await?;
    migration::run().await?;

    Ok(())
}
