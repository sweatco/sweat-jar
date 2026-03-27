use near_workspaces::{types::NearToken, Account};
use nitka::{
    misc::ToNear,
    near_sdk::{
        json_types::{Base64VecU8, U128},
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
        signature: Base64VecU8,
        valid_until: u64,
        timezone: Timezone,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;

    fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: Base64VecU8,
        valid_until: u64,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;

    /// Airdrops `amount_per_receiver` tokens to each account in `receivers`.
    /// Total transferred amount must equal `amount_per_receiver * receivers.len()`.
    /// Caller (`manager`) must be the contract's manager account.
    fn airdrop(
        &self,
        manager: &Account,
        product_id: String,
        amount_per_receiver: u128,
        receivers: &[Account],
        timezone: Option<Timezone>,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;

    /// Same as `airdrop` but for protected products that require an ed25519 signature.
    fn airdrop_protected(
        &self,
        manager: &Account,
        product_id: String,
        amount_per_receiver: u128,
        receivers: &[Account],
        timezone: Option<Timezone>,
        signature: Base64VecU8,
        valid_until: u64,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;
}

pub trait JarContractLegacyExtensions {
    fn create_legacy_step_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        timezone: Timezone,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128>;
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
        signature: Base64VecU8,
        valid_until: u64,
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
                    "valid_until": valid_until.to_string(),
                    "timezone": timezone,
                },
                "signature": signature,
            }
        });

        self.create_jar_internal(user, msg, amount, ft_contract)
    }

    fn create_premium_jar(
        &self,
        user: &Account,
        product_id: String,
        amount: u128,
        signature: Base64VecU8,
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

    fn airdrop(
        &self,
        manager: &Account,
        product_id: String,
        amount_per_receiver: u128,
        receivers: &[Account],
        timezone: Option<Timezone>,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128> {
        let receiver_ids: Vec<&str> = receivers.iter().map(|a| a.id().as_str()).collect();
        let total_amount = amount_per_receiver * receiver_ids.len() as u128;

        println!(
            "▶️ Airdrop(product = {:?}) to {:?} with {:?} tokens each",
            product_id, receiver_ids, amount_per_receiver,
        );

        let msg = json!({
            "type": "airdrop",
            "data": {
                "ticket": {
                    "product_id": product_id,
                    "valid_until": "0",
                    "timezone": timezone,
                },
                "receivers": receiver_ids,
            }
        });

        self.create_jar_internal(manager, msg, total_amount, ft_contract)
    }

    fn airdrop_protected(
        &self,
        manager: &Account,
        product_id: String,
        amount_per_receiver: u128,
        receivers: &[Account],
        timezone: Option<Timezone>,
        signature: Base64VecU8,
        valid_until: u64,
        ft_contract: &SweatContract<'_>,
    ) -> ContractCall<U128> {
        let receiver_ids: Vec<&str> = receivers.iter().map(|a| a.id().as_str()).collect();
        let total_amount = amount_per_receiver * receiver_ids.len() as u128;

        println!(
            "▶️ Airdrop protected(product = {:?}) to {:?} with {:?} tokens each",
            product_id, receiver_ids, amount_per_receiver,
        );

        let msg = json!({
            "type": "airdrop",
            "data": {
                "ticket": {
                    "product_id": product_id,
                    "valid_until": valid_until.to_string(),
                    "timezone": timezone,
                },
                "signature": signature,
                "receivers": receiver_ids,
            }
        });

        self.create_jar_internal(manager, msg, total_amount, ft_contract)
    }
}

impl JarContractLegacyExtensions for SweatJarContract<'_> {
    fn create_legacy_step_jar(
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
                },
            }
        });

        self.create_jar_internal(user, msg, amount, ft_contract)
    }
}
