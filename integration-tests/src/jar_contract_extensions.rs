use anyhow::Result;
use near_workspaces::{types::NearToken, Account};
use nitka::{
    misc::ToNear,
    near_sdk::{
        json_types::U128,
        serde_json::{json, Value},
        Timestamp,
    },
    ContractCall,
};
use sweat_jar_model::{api::SweatJarContract, jar::JarId};
use sweat_model::{FungibleTokenCoreIntegration, SweatContract};

trait Internal {
    fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;
}

impl Internal for SweatJarContract<'_> {
    fn create_jar_internal(
        &self,
        user: &Account,
        msg: Value,
        amount: u128,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128> {
        println!("▶️ Create jar with msg: {:?}", msg,);

        ft_contract
            .ft_transfer_call(
                self.contract.as_account().to_near(),
                amount.into(),
                None,
                msg.to_string(),
            )
            .with_user(user)
            .deposit(NearToken::from_yoctonear(1))
    }
}

pub trait JarContractExtensions {
    fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;

    fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: String,
        valid_until: u64,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;

    fn top_up(
        &self,
        account: &Account,
        jar_id: JarId,
        amount: U128,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;

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
    fn create_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128> {
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

        self.create_jar_internal(user, msg, amount, ft_contract)
    }

    fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: String,
        valid_until: u64,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128> {
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

        self.create_jar_internal(user, msg, amount, ft_contract)
    }

    fn top_up(
        &self,
        account: &Account,
        jar_id: JarId,
        amount: U128,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128> {
        let msg = json!({
            "type": "top_up",
            "data": jar_id,
        });

        println!("▶️ Top up with msg: {:?}", msg,);

        ft_contract
            .ft_transfer_call(self.contract.as_account().to_near(), amount, None, msg.to_string())
            .deposit(NearToken::from_yoctonear(1))
            .with_user(account)
    }

    async fn block_timestamp_ms(&self) -> Result<Timestamp> {
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
