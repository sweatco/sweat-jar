use anyhow::Result;
use near_workspaces::Account;
use nitka::{misc::ToNear, near_sdk::json_types::U128};
use sweat_jar_model::{
    api::{
        InitApiIntegration, IntegrationTestMethodsIntegration, JarApiIntegration, ProductApiIntegration,
        SweatJarContract,
    },
    jar::JarView,
    ProductId,
};
use sweat_model::{FungibleTokenCoreIntegration, StorageManagementIntegration, SweatApiIntegration, SweatContract};

use crate::product::RegisterProductCommand;

pub type Context = nitka::context::Context<near_workspaces::network::Sandbox>;

pub const FT_CONTRACT: &str = "sweat";
pub const SWEAT_JAR: &str = "sweat_jar";

pub trait IntegrationContext {
    async fn manager(&mut self) -> Result<Account>;
    async fn alice(&mut self) -> Result<Account>;
    async fn fee(&mut self) -> Result<Account>;
    fn sweat_jar(&self) -> SweatJarContract;
    fn ft_contract(&self) -> SweatContract;
}

impl IntegrationContext for Context {
    async fn manager(&mut self) -> Result<Account> {
        self.account("manager").await
    }

    async fn alice(&mut self) -> Result<Account> {
        self.account("alice").await
    }

    async fn fee(&mut self) -> Result<Account> {
        self.account("fee").await
    }

    fn sweat_jar(&self) -> SweatJarContract {
        SweatJarContract {
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
    custom_jar_contract: Option<Vec<u8>>,
    products: impl IntoIterator<Item = RegisterProductCommand>,
) -> Result<Context> {
    let mut context = Context::new(&[FT_CONTRACT, SWEAT_JAR], true, "build-integration".into()).await?;

    if let Some(custom_jar) = custom_jar_contract {
        let contract = context
            .sweat_jar()
            .contract
            .as_account()
            .deploy(&custom_jar)
            .await?
            .into_result()?;
        context.contracts.insert(SWEAT_JAR, contract);
    }

    let alice = context.alice().await?;
    let bob = context.account("bob").await?;
    let manager = context.manager().await?;
    let fee_account = context.fee().await?;

    context.ft_contract().new(".u.sweat.testnet".to_string().into()).await?;
    context
        .sweat_jar()
        .init(
            context.ft_contract().contract.as_account().to_near(),
            fee_account.to_near(),
            manager.to_near(),
        )
        .await?;

    context
        .ft_contract()
        .storage_deposit(context.sweat_jar().contract.as_account().to_near().into(), None)
        .await?;

    context
        .ft_contract()
        .tge_mint(&context.sweat_jar().contract.as_account().to_near(), U128(100_000_000))
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
            .register_product(product.get())
            .with_user(&manager)
            .await?;
    }

    Ok(context)
}

pub trait ContextHelpers {
    async fn last_jar_for(&self, account: &Account) -> Result<JarView>;
    async fn bulk_create_jars(
        &mut self,
        account: &Account,
        product_id: &ProductId,
        principal: u128,
        number_of_jars: u16,
    ) -> Result<Vec<JarView>>;
    async fn account_balance(&self, account: &Account) -> Result<u128>;
}

impl ContextHelpers for Context {
    async fn last_jar_for(&self, account: &Account) -> Result<JarView> {
        Ok(self
            .sweat_jar()
            .get_jars_for_account(account.to_near())
            .await?
            .into_iter()
            .last()
            .unwrap())
    }

    async fn bulk_create_jars(
        &mut self,
        account: &Account,
        product_id: &ProductId,
        principal: u128,
        number_of_jars: u16,
    ) -> Result<Vec<JarView>> {
        let total_amount = principal * number_of_jars as u128;

        self.ft_contract()
            .tge_mint(&account.to_near(), U128(100_000_000_000))
            .await?;

        let account_balance = self.account_balance(account).await?;
        assert!(
            account_balance > total_amount,
            r#"
                Account doesn't have enough $SWEAT to create {number_of_jars} jars with {principal} principal.
                Required: {total_amount} has: {account_balance}
            "#,
        );

        self.ft_contract()
            .ft_transfer(
                self.sweat_jar().contract.as_account().id().clone(),
                total_amount.into(),
                None,
            )
            .with_user(account)
            .await?;

        let manager = self.manager().await?;

        self.sweat_jar()
            .bulk_create_jars(account.to_near(), product_id.clone(), principal, number_of_jars)
            .with_user(&manager)
            .await
    }

    async fn account_balance(&self, account: &Account) -> Result<u128> {
        let balance = self.ft_contract().ft_balance_of(account.to_near()).await?.0;
        Ok(balance)
    }
}
