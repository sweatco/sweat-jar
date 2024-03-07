use anyhow::Result;
use near_sdk::{json_types::U128, Timestamp};
use near_workspaces::{types::NearToken, Account};
use serde_json::{json, Value};
use sweat_jar_model::{api::SweatJarContract, jar::JarId};

use crate::measure::outcome_storage::OutcomeStorage;

trait Internal {
    async fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128>;
}

impl Internal for SweatJarContract<'_> {
    async fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128> {
        println!("▶️ Create jar with msg: {:?}", msg,);

        let args = json!({
            "receiver_id": self.contract .as_account().id(),
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

pub trait JarContractExtensions {
    async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128>;

    async fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: String,
        valid_until: u64,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128>;

    async fn top_up(
        &self,
        account: &Account,
        jar_id: JarId,
        amount: U128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128>;

    async fn block_timestamp_ms(&self) -> Result<Timestamp>;

    fn get_signature_material(
        &self,
        receiver_id: &Account,
        product_id: &String,
        valid_until: u64,
        amount: u128,
        last_jar_id: Option<String>,
    ) -> String;
}

impl JarContractExtensions for SweatJarContract<'_> {
    async fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128> {
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
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128> {
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

    async fn top_up(
        &self,
        account: &Account,
        jar_id: JarId,
        amount: U128,
        ft_contract_id: &near_workspaces::AccountId,
    ) -> Result<U128> {
        let msg = json!({
            "type": "top_up",
            "data": jar_id,
        });

        println!("▶️ Top up with msg: {:?}", msg,);

        let args = json!({
            "receiver_id": self.contract .as_account().id(),
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

    async fn block_timestamp_ms(&self) -> anyhow::Result<Timestamp> {
        println!("▶️ block_timestamp_ms");
        let result = self.contract.view("block_timestamp_ms").await?.json()?;
        println!("   ✅ {:?}", result);
        Ok(result)
    }

    fn get_signature_material(
        &self,
        receiver_id: &Account,
        product_id: &String,
        valid_until: u64,
        amount: u128,
        last_jar_id: Option<String>,
    ) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.contract.as_account().id(),
            receiver_id.id(),
            product_id,
            amount,
            last_jar_id.map_or_else(String::new, |value| value),
            valid_until,
        )
    }
}
