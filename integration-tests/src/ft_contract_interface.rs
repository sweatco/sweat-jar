use async_trait::async_trait;
use near_units::parse_near;
use serde_json::json;
use workspaces::{Account, Contract};

#[async_trait]
pub(crate) trait FtContractInterface {
    fn account(&self) -> &Account;

    async fn init(&self) -> anyhow::Result<()>;

    async fn ft_balance_of(&self, user: &Account) -> anyhow::Result<()>;

    async fn mint_for_user(
        &self,
        user: &Account,
        amount: u128,
    ) -> anyhow::Result<()>;

    async fn storage_deposit(&self, user: &Account) -> anyhow::Result<()>;
}

#[async_trait]
impl FtContractInterface for Contract {
    fn account(&self) -> &Account {
        self.as_account()
    }

    async fn init(&self) -> anyhow::Result<()> {
        println!("▶️ Init ft contract");

        self
            .call("new")
            .args_json(json!({
                "postfix": ".u.sweat.testnet",
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn ft_balance_of(&self, user: &Account) -> anyhow::Result<()> {
        println!("▶️ View ft balance of user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        println!("  ▶️ Run ft_balance_of with args {:?}", args);

        let result = user
            .call(self.id(), "ft_balance_of")
            .args_json(args)
            .view()
            .await?;
        println!("    User balance = {:?}", result);

        let parsed_result: String = result.borsh()?;

        println!("    Parsed user balance = {:?}", parsed_result);

        Ok(())
    }

    async fn mint_for_user(
        &self,
        user: &Account,
        amount: u128,
    ) -> anyhow::Result<()> {
        println!("▶️ Mint {:?} tokens for user {:?}", amount, user.id());

        let args = json!({
            "account_id": user.id(),
            "amount": amount.to_string(),
        });

        self
            .as_account()
            .call(self.id(), "tge_mint")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn storage_deposit(&self, user: &Account) -> anyhow::Result<()> {
        println!("▶️ Register {} in ft contract (storage_deposit)", user.id());

        let args = json!({
            "account_id": user.id()
        });

        user.call(self.id(), "storage_deposit")
            .args_json(args)
            .deposit(parse_near!("0.00235 N"))
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }
}
