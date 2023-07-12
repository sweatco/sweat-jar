use async_trait::async_trait;
use near_units::parse_near;
use serde_json::json;
use workspaces::{Account, AccountId, Contract};

#[async_trait]
pub(crate) trait JarContractInterface {
    fn account(&self) -> &Account;
    
    async fn init(
        &self,
        token_contract_account: &Account,
        admin_allowlist: Vec<&AccountId>,
    ) -> anyhow::Result<()>;

    async fn register_product(&self, user: &Account, product_json: serde_json::Value) -> anyhow::Result<()>;

    async fn get_products(&self) -> anyhow::Result<(serde_json::Value)>;

    async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<()>;

    async fn get_principal(&self, user: &Account) -> anyhow::Result<serde_json::Value>;

    async fn get_interest(&self, user: &Account) -> anyhow::Result<serde_json::Value>;
}

#[async_trait]
impl JarContractInterface for Contract {
    fn account(&self) -> &Account {
        self.as_account()
    }

    async fn init(
        &self,
        token_contract_account: &Account,
        admin_allowlist: Vec<&AccountId>,
    ) -> anyhow::Result<()> {
        println!("▶️ Init jar contract");

        self
            .call("init")
            .args_json(json!({
                "token_account_id": token_contract_account.id(),
                "admin_allowlist": admin_allowlist,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn register_product(&self, user: &Account, product_json: serde_json::Value) -> anyhow::Result<()> {
        println!("▶️ Register product: {:?}", product_json);

        let args = json!({
            "product": product_json,
        });

        user.call(self.id(), "register_product")
            .args_json(args)
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn get_products(&self) -> anyhow::Result<(serde_json::Value)> {
        println!("▶️ Get products");

        let products: serde_json::Value = self
            .call("get_products")
            .view()
            .await?
            .json()?;

        println!("@@ Products = {:?}", products);

        Ok(products)
    }

    async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<()> {
        println!(
            "▶️ Create jar(product = {:?}) for user {:?} with {:?} tokens",
            product_id,
            user.id(),
            amount
        );

        let msg = json!({
            "Stake": {
                "product_id": product_id,
            }
        });

        let args = json!({
            "receiver_id": self.as_account().id(),
            "amount": amount.to_string(),
            "msg": msg.to_string(),
        });

        user.call(ft_contract_id, "ft_transfer_call")
            .args_json(args)
            .max_gas()
            .deposit(parse_near!("1 yocto"))
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn get_principal(&self, user: &Account) -> anyhow::Result<serde_json::Value> {
        println!("▶️ Get total principal for user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result: serde_json::Value = self
            .call("get_principal")
            .args_json(args)
            .view()
            .await?
            .json()?;

        Ok(result)
    }

    async fn get_interest(&self, user: &Account) -> anyhow::Result<serde_json::Value> {
        println!("▶️ Get total interest for user {:?}", user.id());

        let args = json!({
        "account_id": user.id(),
    });

        let result: serde_json::Value = self
            .call("get_interest")
            .args_json(args)
            .view()
            .await?
            .json()?;

        Ok(result)
    }
}
