use anyhow::Result;
use async_trait::async_trait;
use integration_utils::integration_contract::IntegrationContract;
use model::{
    api::{
        ClaimApiIntegration, InitApiIntegration, JarApiIntegration, PenaltyApiIntegration, ProductApiIntegration,
        WithdrawApiIntegration,
    },
    jar::{AggregatedInterestView, AggregatedTokenAmountView, JarId, JarIdView, JarView},
    product::{ProductView, RegisterProductCommand},
    withdraw::WithdrawView,
    ProductId,
    AggregatedTokenAmountView,
};
use near_sdk::{
    json_types::{Base64VecU8, U128},
    AccountId, Timestamp,
};
use near_workspaces::{types::NearToken, Account, Contract};
use serde_json::{json, Value};

use crate::measure::outcome_storage::OutcomeStorage;

pub const SWEAT_JAR: &str = "sweat_jar";

pub struct SweatJar<'a> {
    contract: &'a Contract,
    account: Option<Account>,
}

#[async_trait]
impl InitApiIntegration for SweatJar<'_> {
    async fn init(&self, token_account_id: AccountId, fee_account_id: AccountId, manager: AccountId) -> Result<()> {
        println!("▶️ Init jar contract");

        self.contract()
            .call("init")
            .args_json(json!({
                "token_account_id": token_account_id,
                "fee_account_id": fee_account_id,
                "manager": manager,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }
}

#[async_trait]
impl ClaimApiIntegration for SweatJar<'_> {
    async fn claim_total(&mut self) -> Result<U128> {
        println!("▶️ Claim total");

        let args = json!({});

        let result = self
            .user_account()
            .call(self.contract().id(), "claim_total")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        println!("   📟 {result:#?}");

        for failure in result.failures() {
            println!("   ❌ {:?}", failure);
        }

        if let Some(failure) = result.failures().into_iter().next().cloned() {
            let error = failure.into_result().err().unwrap();
            return Err(error.into());
        }

        let ret = result.json()?;

        OutcomeStorage::add_result(result);

        Ok(ret)
    }

    async fn claim_jars(&mut self, jar_ids: Vec<JarIdView>, amount: Option<U128>) -> Result<U128> {
        println!("▶️ Claim jars: {:?}", jar_ids);
    async fn claim_total_detailed(&self, user: &Account) -> anyhow::Result<AggregatedTokenAmountView>;

        let args = json!({
            "jar_ids": jar_ids,
            "amount": amount,
        });
    async fn claim_jars(&self, user: &Account, jar_ids: Vec<JarIdView>, amount: Option<U128>) -> anyhow::Result<U128>;

    async fn get_jar(&self, account_id: String, jar_id: JarIdView) -> anyhow::Result<JarView>;

        let result = self
            .user_account()
            .call(self.contract.id(), "claim_jars")
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
}

#[async_trait]
impl WithdrawApiIntegration for SweatJar<'_> {
    async fn withdraw(&mut self, jar_id: JarIdView, amount: Option<U128>) -> Result<WithdrawView> {
        println!("▶️ Withdraw jar #{jar_id:?}");

        let args = json!({
            "jar_id": jar_id,
            "amount": amount,
        });

        let result = self
            .user_account()
            .call(self.contract.id(), "withdraw")
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
}

#[async_trait]
impl JarApiIntegration for SweatJar<'_> {
    async fn get_jar(&self, account_id: AccountId, jar_id: JarIdView) -> Result<JarView> {
        println!("▶️ Get jar #{jar_id:?}");

        let args = json!({
            "account_id": account_id,
            "jar_id": jar_id,
        });

        let result = self.contract.view("get_jar").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_jars_for_account(&self, account_id: AccountId) -> Result<Vec<JarView>> {
        println!("▶️ Get jars for user {:?}", account_id);

        let args = json!({
            "account_id": account_id,
        });

        let result = self
            .contract
            .view("get_jars_for_account")
            .args_json(args)
            .await?
            .json()?;

        println!("   ✅ {result:?}");

        Ok(result)
    }

    async fn get_total_principal(&self, account_id: AccountId) -> Result<AggregatedTokenAmountView> {
        println!("▶️ Get total principal for user {:?}", account_id);

        let args = json!({
            "account_id": account_id,
        });

        let result = self
            .contract
            .view("get_total_principal")
            .args_json(args)
            .await?
            .json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_principal(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> Result<AggregatedTokenAmountView> {
        println!("▶️ Get principal for jars {:?}", jar_ids);

        let args = json!({
            "account_id": account_id,
            "jar_ids": jar_ids,
        });

        let result = self.contract.view("get_principal").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_total_interest(&self, account_id: AccountId) -> Result<AggregatedInterestView> {
        println!("▶️ Get total interest for user {:?}", account_id);

        let args = json!({
            "account_id": account_id,
        });

        let result = self.contract.view("get_total_interest").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn get_interest(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> Result<AggregatedInterestView> {
        println!("▶️ Get interest for jars {:?}", jar_ids);

        let args = json!({
            "account_id": account_id,
            "jar_ids": jar_ids,
        });

        let result = self.contract.view("get_interest").args_json(args).await?.json()?;

        println!("   ✅ {:?}", result);

        Ok(result)
    }

    async fn restake(&mut self, jar_id: JarIdView) -> Result<JarView> {
        println!("▶️ Restake jar #{jar_id:?}");

        let args = json!({
            "jar_id": jar_id,
        });

        let result = self
            .user_account()
            .call(self.contract.id(), "restake")
            .args_json(args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        let view = result.json()?;

        OutcomeStorage::add_result(result);

        Ok(view)
    }
}

#[async_trait]
impl PenaltyApiIntegration for SweatJar<'_> {
    async fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool) -> Result<()> {
        println!("▶️ Set penalty for jar #{jar_id:?}");

        let args = json!({
            "account_id": account_id,
            "jar_id": jar_id,
            "value": value,
        });

        let result = self
            .user_account()
            .call(self.contract.id(), "set_penalty")
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

    async fn batch_set_penalty(&mut self, _jars: Vec<(AccountId, Vec<JarIdView>)>, _value: bool) -> Result<()> {
        todo!()
    }
}

#[async_trait]
impl ProductApiIntegration for SweatJar<'_> {
    async fn register_product(&mut self, command: RegisterProductCommand) -> Result<()> {
        println!("▶️ Register product: {command:?}");
    async fn claim_total_detailed(&self, user: &Account) -> anyhow::Result<AggregatedTokenAmountView> {
        println!("▶️ Claim total detailed");

        let args = json!({
            "detailed": true
        });

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

        let result_value: AggregatedTokenAmountView = result.json()?;

        println!("   ✅ {result_value:?}");

        OutcomeStorage::add_result(result);

        Ok(result_value)
    }

    async fn claim_jars(&self, user: &Account, jar_ids: Vec<JarIdView>, amount: Option<U128>) -> anyhow::Result<U128> {
        println!("▶️ Claim jars: {:?}", jar_ids);

        let args = json!({
            "command": command,
        });

        let result = self
            .user_account()
            .call(self.contract.id(), "register_product")
            .args_json(args)
            .deposit(NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        println!("   📟 {result:#?}");

        let result_value: Value = result.json()?;

        println!("   ✅ {result_value:?}");

        OutcomeStorage::add_result(result);

        Ok(())
        Ok(serde_json::from_value(result_value).unwrap())
    }

    async fn set_enabled(&mut self, product_id: ProductId, is_enabled: bool) -> Result<()> {
        println!("▶️ Set enabled for product #{product_id}");

        let args = json!({
            "product_id": product_id,
            "is_enabled": is_enabled,
        });

        let result = self
            .user_account()
            .call(self.contract.id(), "set_enabled")
            .args_json(args)
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        Ok(())
    }

    async fn set_public_key(&mut self, product_id: ProductId, public_key: Base64VecU8) -> Result<()> {
        println!("▶️ Set public key for product #{product_id}: {public_key:?}");

        let args = json!({
            "product_id": product_id,
            "public_key": public_key,
        });

        println!("Args: {:?}", args);

        let result = self
            .user_account()
            .call(self.contract.id(), "set_public_key")
            .args_json(args)
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()?;

        for log in result.logs() {
            println!("   📖 {log}");
        }

        Ok(())
    }

    async fn get_products(&self) -> Result<Vec<ProductView>> {
        println!("▶️ Get products");

        let products = self.contract.view("get_products").await?.json()?;

        println!("   ✅ {:?}", products);

        Ok(products)
    }
}

#[async_trait]
pub(crate) trait JarContractInterface {
    async fn batch_set_penalty(
        &mut self,
        admin: &Account,
        jars: Vec<(AccountId, Vec<JarIdView>)>,
        value: bool,
    ) -> anyhow::Result<()>;
}

#[async_trait]
impl JarContractInterface for Contract {
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
}

#[async_trait]
trait Internal {
    async fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> anyhow::Result<U128>;
}

#[async_trait]
impl Internal for SweatJar<'_> {
    async fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> anyhow::Result<U128> {
        println!("▶️ Create jar with msg: {:?}", msg,);

        let args = json!({
            "receiver_id": self.contract() .as_account().id(),
            "amount": amount.to_string(),
            "msg": msg.to_string(),
        });

        let result = user
            .call(ft_contract_id, "ft_transfer_call")
            .args_json(args)
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
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

impl SweatJar<'_> {
    pub async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &near_workspaces::AccountId,
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

    pub async fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: String,
        valid_until: u64,
        ft_contract_id: &near_workspaces::AccountId,
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

    pub async fn top_up(
        &self,
        account: &Account,
        jar_id: JarId,
        amount: U128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> anyhow::Result<U128> {
        let msg = json!({
            "type": "top_up",
            "data": jar_id,
        });

        println!("▶️ Top up with msg: {:?}", msg,);

        let args = json!({
            "receiver_id": self.contract() .as_account().id(),
            "amount": amount.0.to_string(),
            "msg": msg.to_string(),
        });

        let result = account
            .call(ft_contract_id, "ft_transfer_call")
            .args_json(args)
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
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

    pub(crate) async fn block_timestamp_ms(&self) -> anyhow::Result<Timestamp> {
        println!("▶️ block_timestamp_ms");
        let result = self.contract.view("block_timestamp_ms").await?.json()?;
        println!("   ✅ {:?}", result);
        Ok(result)
    }

    pub(crate) fn get_signature_material(
        &self,
        receiver_id: &Account,
        product_id: &String,
        valid_until: u64,
        amount: u128,
        last_jar_id: Option<String>,
    ) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.contract().as_account().id(),
            receiver_id.id(),
            product_id,
            amount,
            last_jar_id.map_or_else(String::new, |value| value,),
            valid_until,
        )
    }
}

impl<'a> IntegrationContract<'a> for SweatJar<'a> {
    fn with_contract(contract: &'a Contract) -> Self {
        Self {
            contract,
            account: None,
        }
    }

    fn with_user(&mut self, account: &Account) -> &mut Self {
        self.account = account.clone().into();
        self
    }

    fn user_account(&self) -> Account {
        self.account
            .as_ref()
            .expect("Set account with `user` method first")
            .clone()
    }

    fn contract(&self) -> &'a Contract {
        self.contract
    }
}
