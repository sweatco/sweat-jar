use async_trait::async_trait;
use integration_utils::{integration_contract::IntegrationContract, misc::ToNear};
use model::api::{InitApiIntegration, ProductApiIntegration};
use near_sdk::json_types::U128;
use near_workspaces::Account;
use sweat_integration::{SweatFt, FT_CONTRACT};
use sweat_model::{StorageManagementIntegration, SweatApiIntegration};

use crate::{
    jar_contract_interface::{SweatJar, SWEAT_JAR},
    product::RegisterProductCommand,
};

pub type Context = integration_utils::context::Context<near_workspaces::network::Sandbox>;

#[async_trait]
pub trait IntegrationContext {
    async fn manager(&mut self) -> anyhow::Result<Account>;
    async fn alice(&mut self) -> anyhow::Result<Account>;
    async fn fee(&mut self) -> anyhow::Result<Account>;
    fn sweat_jar(&self) -> SweatJar;
    fn ft_contract(&self) -> SweatFt;
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

    fn sweat_jar(&self) -> SweatJar {
        SweatJar::with_contract(&self.contracts[SWEAT_JAR])
    }

    fn ft_contract(&self) -> SweatFt {
        SweatFt::with_contract(&self.contracts[FT_CONTRACT])
    }
}

pub(crate) async fn prepare_contract(
    products: impl IntoIterator<Item = RegisterProductCommand>,
) -> anyhow::Result<Context> {
    let mut context = Context::new(&[FT_CONTRACT, SWEAT_JAR], "build-integration".into()).await?;

    let alice = context.account("alice").await?;
    let bob = context.account("bob").await?;
    let manager = context.account("manager").await?;
    let fee_account = context.account("fee").await?;

    context.ft_contract().new(".u.sweat.testnet".to_string().into()).await?;
    context
        .sweat_jar()
        .init(
            context.ft_contract().contract().as_account().to_near(),
            fee_account.to_near(),
            manager.to_near(),
        )
        .await?;

    context
        .ft_contract()
        .storage_deposit(context.sweat_jar().contract().as_account().to_near().into(), None)
        .await?;

    context
        .ft_contract()
        .tge_mint(
            &context.sweat_jar().contract().as_account().to_near(),
            U128(100_000_000),
        )
        .await?;

    context
        .ft_contract()
        .storage_deposit(fee_account.to_near().into(), None)
        .await?;
    context
        .ft_contract()
        .storage_deposit(alice.to_near().into(), None)
        .await?;
    context
        .ft_contract()
        .tge_mint(&alice.to_near(), U128(100_000_000))
        .await?;
    context
        .ft_contract()
        .storage_deposit(bob.to_near().into(), None)
        .await?;
    context
        .ft_contract()
        .tge_mint(&bob.to_near(), U128(100_000_000))
        .await?;
    context
        .ft_contract()
        .tge_mint(&manager.to_near(), U128(100_000_000))
        .await?;

    for product in products {
        context
            .sweat_jar()
            .with_user(&manager)
            .register_product(product.get())
            .await?;
    }

    Ok(context)
}
