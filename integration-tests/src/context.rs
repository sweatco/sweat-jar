use async_trait::async_trait;
use integration_utils::misc::ToNear;
use jar_model::api::{InitApiIntegration, JarContract, ProductApiIntegration};
use near_sdk::json_types::U128;
use near_workspaces::Account;
use sweat_model::{StorageManagementIntegration, SweatApiIntegration, SweatContract};

use crate::product::RegisterProductCommand;

pub type Context = integration_utils::context::Context<near_workspaces::network::Sandbox>;

pub const FT_CONTRACT: &str = "sweat";
pub const SWEAT_JAR: &str = "sweat_jar";

#[async_trait]
pub trait IntegrationContext {
    async fn manager(&mut self) -> anyhow::Result<Account>;
    async fn alice(&mut self) -> anyhow::Result<Account>;
    async fn fee(&mut self) -> anyhow::Result<Account>;
    fn sweat_jar(&self) -> JarContract;
    fn ft_contract(&self) -> SweatContract;
}

#[async_trait]
impl IntegrationContext for Context {
    async fn manager(&mut self) -> anyhow::Result<Account> {
        self.account("manager").await
    }

    async fn alice(&mut self) -> anyhow::Result<Account> {
        self.account("alice").await
    }

    async fn fee(&mut self) -> anyhow::Result<Account> {
        self.account("fee").await
    }

    fn sweat_jar(&self) -> JarContract {
        JarContract {
            contract: &self.contracts[SWEAT_JAR],
        }
    }

    fn ft_contract(&self) -> SweatContract {
        SweatContract {
            contract: &self.contracts[FT_CONTRACT],
        }
    }
}

pub(crate) async fn prepare_contract(
    products: impl IntoIterator<Item = RegisterProductCommand>,
) -> anyhow::Result<Context> {
    let mut context = Context::new(&[FT_CONTRACT, SWEAT_JAR], true, "build-integration".into()).await?;

    let alice = context.account("alice").await?;
    let bob = context.account("bob").await?;
    let manager = context.account("manager").await?;
    let fee_account = context.account("fee").await?;

    context
        .ft_contract()
        .new(".u.sweat.testnet".to_string().into())
        .call()
        .await?;
    context
        .sweat_jar()
        .init(
            context.ft_contract().contract.as_account().to_near(),
            fee_account.to_near(),
            manager.to_near(),
        )
        .call()
        .await?;

    context
        .ft_contract()
        .storage_deposit(context.sweat_jar().contract.as_account().to_near().into(), None)
        .call()
        .await?;

    context
        .ft_contract()
        .tge_mint(&context.sweat_jar().contract.as_account().to_near(), U128(100_000_000))
        .call()
        .await?;

    context
        .ft_contract()
        .storage_deposit(fee_account.to_near().into(), None)
        .call()
        .await?;
    context
        .ft_contract()
        .storage_deposit(alice.to_near().into(), None)
        .call()
        .await?;
    context
        .ft_contract()
        .tge_mint(&alice.to_near(), U128(100_000_000))
        .call()
        .await?;
    context
        .ft_contract()
        .storage_deposit(bob.to_near().into(), None)
        .call()
        .await?;
    context
        .ft_contract()
        .tge_mint(&bob.to_near(), U128(100_000_000))
        .call()
        .await?;
    context
        .ft_contract()
        .tge_mint(&manager.to_near(), U128(100_000_000))
        .call()
        .await?;

    for product in products {
        context
            .sweat_jar()
            .register_product(product.get())
            .with_user(&manager)
            .call()
            .await?;
    }

    Ok(context)
}
