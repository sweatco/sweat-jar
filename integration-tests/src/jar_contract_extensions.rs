use near_workspaces::{types::NearToken, Account};
use nitka::{
    misc::ToNear,
    near_sdk::{
        json_types::U128,
        serde_json::{json, Value},
    },
    ContractCall,
};
use sweat_jar_model::{api::SweatJarContract, Timezone};
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

    fn create_step_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        timezone: Timezone,
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

    fn get_signature_material(
        &self,
        receiver_id: &Account,
        product_id: &String,
        valid_until: u64,
        amount: u128,
        nonce: u32,
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

    fn create_step_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        timezone: Timezone,
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
                    "timezone": timezone,
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

    fn get_signature_material(
        &self,
        receiver_id: &Account,
        product_id: &String,
        valid_until: u64,
        amount: u128,
        nonce: u32,
    ) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.contract.as_account().id(),
            receiver_id.id(),
            product_id,
            amount,
            nonce,
            valid_until,
        )
    }
}
