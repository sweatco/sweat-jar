use async_trait::async_trait;
use model::{
    jar::{JarId, JarIdView, JarView},
    withdraw::WithdrawView,
};
use near_sdk::json_types::U128;
use near_units::parse_near;
use near_workspaces::{Account, AccountId, Contract};
use serde_json::{json, Value};

use crate::measure::outcome_storage::OutcomeStorage;

#[async_trait]
pub(crate) trait JarContractInterface {
    fn account(&self) -> &Account;

    async fn init(
        &self,
        token_contract_account: &Account,
        fee_account: &Account,
        manager: &AccountId,
    ) -> anyhow::Result<()>;

    async fn register_product(&self, user: &Account, register_product_command_json: Value) -> anyhow::Result<()>;

    async fn get_products(&self) -> anyhow::Result<Value>;

    async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128>;

    async fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: String,
        valid_until: u64,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128>;

    async fn get_total_principal(&self, user: &Account) -> anyhow::Result<Value>;

    async fn get_principal(&self, user: &Account, jar_ids: Vec<JarIdView>) -> anyhow::Result<Value>;

    async fn get_total_interest(&self, user: &Account) -> anyhow::Result<Value>;

    async fn get_interest(&self, user: &Account, jar_ids: Vec<JarIdView>) -> anyhow::Result<Value>;

    async fn get_jars_for_account(&self, user: &Account) -> anyhow::Result<Vec<JarView>>;

    async fn withdraw(&self, user: &Account, jar_id: JarIdView) -> anyhow::Result<WithdrawView>;

    async fn claim_total(&self, user: &Account) -> anyhow::Result<u128>;

    async fn claim_jars(&self, user: &Account, jar_ids: Vec<JarIdView>, amount: Option<U128>) -> anyhow::Result<u128>;

    async fn get_jar(&self, account_id: String, jar_id: JarIdView) -> anyhow::Result<JarView>;

    async fn restake(&self, user: &Account, jar_id: JarIdView) -> anyhow::Result<()>;

    async fn set_penalty(
        &self,
        admin: &Account,
        account_id: &str,
        jar_id: JarIdView,
        value: bool,
    ) -> anyhow::Result<()>;

    async fn batch_set_penalty(
        &mut self,
        admin: &Account,
        jars: Vec<(AccountId, Vec<JarIdView>)>,
        value: bool,
    ) -> anyhow::Result<()>;

    async fn set_enabled(&self, admin: &Account, product_id: String, is_enabled: bool) -> anyhow::Result<()>;

    async fn set_public_key(&self, admin: &Account, product_id: String, public_key: String) -> anyhow::Result<()>;

    async fn top_up(
        &self,
        account: &Account,
        jar_id: JarId,
        amount: U128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128>;

    async fn get_coverage(&self) -> anyhow::Result<Vec<u8>>;
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

        self.call("init")
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

    async fn register_product(&self, manager: &Account, register_product_command_json: Value) -> anyhow::Result<()> {
        println!("▶️ Register product: {register_product_command_json:?}");

        let args = json!({
            "command": register_product_command_json,
        });

        let result = manager
            .call(self.id(), "register_product")
            .args_json(args)
            .deposit(1)
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        OutcomeStorage::add_result(result);

        Ok(())
    }

    async fn get_products(&self) -> anyhow::Result<Value> {
        println!("▶️ Get products");

        let products: Value = self.view("get_products").await?.json()?;

        println!("   ✅ {:?}", products);

        Ok(products)
    }

    async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128> {
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

        self.create_jar_internal(user, msg, amount, ft_contract_id).await
    }

    async fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: String,
        valid_until: u64,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128> {
        println!(
            "▶️ Create premium jar(product = {:?}) for user {:?} with {:?} tokens",
            product_id,
            user.id(),
            amount
        );

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": product_id,
                    "valid_until": valid_until.to_string(),
                },
                "signature": signature,
            }
        });

        self.create_jar_internal(user, msg, amount, ft_contract_id).await
    }

    async fn get_total_principal(&self, user: &Account) -> anyhow::Result<Value> {
        println!("▶️ Get total principal for user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result = self.view("get_total_principal").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_principal(&self, user: &Account, jar_ids: Vec<JarIdView>) -> anyhow::Result<Value> {
        println!("▶️ Get principal for jars {:?}", jar_ids);

        let args = json!({
            "account_id": user.id(),
            "jar_ids": jar_ids,
        });

        let result = self.view("get_principal").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_total_interest(&self, user: &Account) -> anyhow::Result<Value> {
        println!("▶️ Get total interest for user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result = self.view("get_total_interest").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_interest(&self, user: &Account, jar_ids: Vec<JarIdView>) -> anyhow::Result<Value> {
        println!("▶️ Get interest for jars {:?}", jar_ids);

        let args = json!({
            "account_id": user.id(),
            "jar_ids": jar_ids,
        });

        let result = self.view("get_interest").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_jars_for_account(&self, user: &Account) -> anyhow::Result<Vec<JarView>> {
        println!("▶️ Get jars for user {:?}", user.id());

        let args = json!({
            "account_id": user.id(),
        });

        let result = self.view("get_jars_for_account").args_json(args).await?.json()?;

        println!("   ✅ {result:?}");

        Ok(result)
    }

    async fn withdraw(&self, user: &Account, jar_id: JarIdView) -> anyhow::Result<WithdrawView> {
        println!("▶️ Withdraw jar #{jar_id:?}");

        let args = json!({
            "jar_id": jar_id,
        });

        let result = user
            .call(self.id(), "withdraw")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        println!("   📟 {result:#?}");

        let result_value = result.json()?;

        println!("   ✅ {result_value:?}");

        OutcomeStorage::add_result(result);

        Ok(result_value)
    }

    async fn claim_total(&self, user: &Account) -> anyhow::Result<u128> {
        println!("▶️ Claim total");

        let args = json!({});

        let result = user
            .call(self.id(), "claim_total")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        println!("   📟 {result:#?}");

        let result_value = result.json::<Value>()?;

        println!("   ✅ {result_value:?}");

        for failure in result.failures() {
            println!("   ❌ {:?}", failure);
        }

        if let Some(failure) = result.failures().into_iter().next().cloned() {
            let error = failure.into_result().err().unwrap();
            return Err(error.into());
        }

        OutcomeStorage::add_result(result);

        Ok(result_value.as_str().unwrap().to_string().parse::<u128>()?)
    }

    async fn claim_jars(&self, user: &Account, jar_ids: Vec<JarIdView>, amount: Option<U128>) -> anyhow::Result<u128> {
        println!("▶️ Claim jars: {:?}", jar_ids);

        let args = json!({
            "jar_ids": jar_ids,
            "amount": amount,
        });

        let result = user
            .call(self.id(), "claim_jars")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        println!("   📟 {result:#?}");

        let result_value = result.json::<Value>()?;

        println!("   ✅ {result_value:?}");

        OutcomeStorage::add_result(result);

        Ok(result_value.as_str().unwrap().to_string().parse::<u128>()?)
    }

    async fn get_jar(&self, account_id: String, jar_id: JarIdView) -> anyhow::Result<JarView> {
        println!("▶️ Get jar #{jar_id:?}");

        let args = json!({
            "account_id": account_id,
            "jar_id": jar_id,
        });

        let result = self.view("get_jar").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn restake(&self, user: &Account, jar_id: JarIdView) -> anyhow::Result<()> {
        println!("▶️ Restake jar #{jar_id:?}");

        let args = json!({
            "jar_id": jar_id,
        });

        let result = user
            .call(self.id(), "restake")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        OutcomeStorage::add_result(result);

        Ok(())
    }

    async fn set_penalty(
        &self,
        admin: &Account,
        account_id: &str,
        jar_id: JarIdView,
        value: bool,
    ) -> anyhow::Result<()> {
        println!("▶️ Set penalty for jar #{jar_id:?}");

        let args = json!({
            "account_id": account_id,
            "jar_id": jar_id,
            "value": value,
        });

        let result = admin
            .call(self.id(), "set_penalty")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        Ok(())
    }

    async fn batch_set_penalty(
        &mut self,
        admin: &Account,
        jars: Vec<(AccountId, Vec<JarIdView>)>,
        value: bool,
    ) -> anyhow::Result<()> {
        println!("▶️ batch_set_penalty");

        let args = json!({
            "jars": jars,
            "value": value,
        });

        let result = admin
            .call(self.id(), "batch_set_penalty")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        OutcomeStorage::add_result(result);

        Ok(())
    }

    async fn set_enabled(&self, admin: &Account, product_id: String, is_enabled: bool) -> anyhow::Result<()> {
        println!("▶️ Set enabled for product #{product_id}");

        let args = json!({
            "product_id": product_id,
            "is_enabled": is_enabled,
        });

        let result = admin
            .call(self.id(), "set_enabled")
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

    async fn set_public_key(&self, admin: &Account, product_id: String, public_key: String) -> anyhow::Result<()> {
        println!("▶️ Set public key for product #{product_id}: {public_key}");

        let args = json!({
            "product_id": product_id,
            "public_key": public_key,
        });

        println!("Args: {:?}", args);

        let result = admin
            .call(self.id(), "set_public_key")
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

    async fn top_up(
        &self,
        account: &Account,
        jar_id: JarId,
        amount: U128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128> {
        let msg = json!({
            "type": "top_up",
            "data": jar_id,
        });

        println!("▶️ Top up with msg: {:?}", msg,);

        let args = json!({
            "receiver_id": self.as_account().id(),
            "amount": amount.0.to_string(),
            "msg": msg.to_string(),
        });

        let result = account
            .call(ft_contract_id, "ft_transfer_call")
            .args_json(args)
            .max_gas()
            .deposit(parse_near!("1 yocto"))
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        for failure in result.failures() {
            println!("   ❌ {:?}", failure);
        }

        if let Some(failure) = result.failures().into_iter().next().cloned() {
            let error = failure.into_result().err().unwrap();
            return Err(error.into());
        }

        let result_value = result.json()?;

        OutcomeStorage::add_result(result);

        Ok(result_value)
    }

    async fn get_coverage(&self) -> anyhow::Result<Vec<u8>> {
        println!("▶️ Get coverage");

        let result = self.view("get_coverage").await?.result;

        Ok(result)
    }
}

#[async_trait]
trait Internal {
    async fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128>;
}

#[async_trait]
impl Internal for Contract {
    async fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract_id: &AccountId,
    ) -> anyhow::Result<U128> {
        println!("▶️ Create jar with msg: {:?}", msg,);

        let args = json!({
            "receiver_id": self.as_account().id(),
            "amount": amount.to_string(),
            "msg": msg.to_string(),
        });

        let result = user
            .call(ft_contract_id, "ft_transfer_call")
            .args_json(args)
            .max_gas()
            .deposit(parse_near!("1 yocto"))
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        for failure in result.failures() {
            println!("   ❌ {:?}", failure);
        }

        if let Some(failure) = result.failures().into_iter().next().cloned() {
            let error = failure.into_result().err().unwrap();
            return Err(error.into());
        }

        let result_value = result.json()?;

        OutcomeStorage::add_result(result);

        Ok(result_value)
    }
}
