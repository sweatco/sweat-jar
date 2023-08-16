use async_trait::async_trait;
use near_sdk::json_types::U128;
use near_units::parse_near;
use serde_json::{json, Value};
use workspaces::{Account, AccountId, Contract};

#[async_trait]
pub(crate) trait JarContractInterface {
    fn account(&self) -> &Account;

    async fn init(
        &self,
        token_contract_account: &Account,
        fee_account: &Account,
        manager: &AccountId,
    ) -> anyhow::Result<()>;

    async fn register_product(
        &self,
        user: &Account,
        register_product_command_json: Value,
    ) -> anyhow::Result<()>;

    async fn get_products(&self) -> anyhow::Result<(Value)>;

    async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<()>;

    async fn get_total_principal(&self, user: &Account) -> anyhow::Result<U128>;

    async fn get_total_interest(&self, user: &Account) -> anyhow::Result<U128>;

    async fn get_jars_for_account(&self, user: &Account) -> anyhow::Result<Value>;

    async fn withdraw(&self, user: &Account, jar_index: u32) -> anyhow::Result<()>;

    async fn time(&self) -> anyhow::Result<u64>;

    async fn claim_total(&self, user: &Account) -> anyhow::Result<()>;
}

#[async_trait]
impl JarContractInterface for Contract {
    fn account(&self) -> &Account {
        self.as_account()
    }

    async fn init(
        &self,
        token_contract_account: &Account,
        fee_account: &Account,
        manager: &AccountId,
    ) -> anyhow::Result<()> {
        println!("▶️ Init jar contract");

        self
            .call("init")
            .args_json(json!({
                "token_account_id": token_contract_account.id(),
                "fee_account_id": fee_account.id(),
                "manager": manager,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn register_product(
        &self,
        user: &Account,
        register_product_command_json: Value,
    ) -> anyhow::Result<()> {
        println!("▶️ Register product: {:?}", register_product_command_json);

        let args = json!({
            "command": register_product_command_json,
        });

        let result = user.call(self.id(), "register_product")
            .args_json(args)
            .deposit(1)
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {:?}", log);
        }

        Ok(())
    }

    async fn get_products(&self) -> anyhow::Result<(Value)> {
        println!("▶️ Get products");

        let products: Value = self
            .view("get_products")
            .await?
            .json()?;

        println!("   ✅ {:?}", products);

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
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": product_id,
                    "valid_until": "0",
                }
            }
        });

        let args = json!({
            "receiver_id": self.as_account().id(),
            "amount": amount.to_string(),
            "msg": msg.to_string(),
        });

        let result = user.call(ft_contract_id, "ft_transfer_call")
            .args_json(args)
            .max_gas()
            .deposit(parse_near!("1 yocto"))
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {:?}", log);
        }

        Ok(())
    }

    async fn get_total_principal(&self, user: &Account) -> anyhow::Result<U128> {
        println!("▶️ Get total principal for user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result = self
            .view("get_total_principal")
            .args_json(args)
            .await?
            .json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_total_interest(&self, user: &Account) -> anyhow::Result<U128> {
        println!("▶️ Get total interest for user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result = self
            .view("get_total_interest")
            .args_json(args)
            .await?
            .json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_jars_for_account(&self, user: &Account) -> anyhow::Result<Value> {
        println!("▶️ Get jars for user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result = self
            .view("get_jars_for_account")
            .args_json(args)
            .await?
            .json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn withdraw(&self, user: &Account, jar_index: u32) -> anyhow::Result<()> {
        println!("▶️ Withdraw jar #{}", jar_index);

        let args = json!({
            "jar_index": jar_index,
        });

        let result = user.call(self.id(), "withdraw")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {:?}", log);
        }

        println!("   📟 {:?}", result);

        Ok(())
    }

    async fn time(&self) -> anyhow::Result<u64> {
        println!("▶️ Get current block time");

        let result: Value = self
            .call("time")
            .view()
            .await?
            .json()?;

        Ok(result.as_u64().unwrap())
    }

    async fn claim_total(&self, user: &Account) -> anyhow::Result<()> {
        println!("▶️ Claim total");

        let args = json!({});

        let result = user.call(self.id(), "claim_total")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {:?}", log);
        }

        println!("   📟 {:?}", result);

        Ok(())
    }
}
