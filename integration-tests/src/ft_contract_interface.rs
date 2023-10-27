use async_trait::async_trait;
use near_sdk::json_types::U128;
use near_units::parse_near;
use near_workspaces::{Account, AccountId, Contract};
use serde_json::json;

#[async_trait]
pub(crate) trait FtContractInterface {
    fn account(&self) -> &Account;

    async fn init(&self) -> anyhow::Result<()>;

    async fn ft_balance_of(&self, user: &Account) -> anyhow::Result<U128>;

    async fn mint_for_user(&self, user: &Account, amount: u128) -> anyhow::Result<()>;

    async fn storage_deposit(&self, user: &Account) -> anyhow::Result<()>;

    async fn ft_transfer_call(
        &self,
        account: &Account,
        receiver_id: &AccountId,
        amount: u128,
        msg: String,
    ) -> anyhow::Result<()>;
}

#[async_trait]
impl FtContractInterface for Contract {
    fn account(&self) -> &Account {
        self.as_account()
    }

    async fn init(&self) -> anyhow::Result<()> {
        println!("▶️ Init ft contract");

        self.call("new")
            .args_json(json!({
                "postfix": ".u.sweat.testnet",
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn ft_balance_of(&self, user: &Account) -> anyhow::Result<U128> {
        println!("▶️ View ft balance of user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result = self.view("ft_balance_of").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn mint_for_user(&self, user: &Account, amount: u128) -> anyhow::Result<()> {
        println!("▶️ Mint {amount} tokens for user {}", user.id());

        let args = json!({
            "account_id": user.id(),
            "amount": amount.to_string(),
        });

        self.as_account()
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

    async fn ft_transfer_call(
        &self,
        user: &Account,
        receiver_id: &AccountId,
        amount: u128,
        msg: String,
    ) -> anyhow::Result<()> {
        println!("▶️ Transfer {amount} fungible tokens to {receiver_id} with message: {msg}");

        let args = json!({
            "receiver_id": receiver_id,
            "amount": amount.to_string(),
            "msg": msg.to_string(),
        });

        let result = user
            .call(self.id(), "ft_transfer_call")
            .args_json(args)
            .max_gas()
            .deposit(parse_near!("1 yocto"))
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        Ok(())
    }
}
