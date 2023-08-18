use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{json_types::U128, PromiseOrValue, serde::{Deserialize, Serialize}, serde_json};

use crate::*;
use crate::jar::model::JarTicket;
use crate::migration::model::CeFiJar;

/// The `FtMessage` enum represents various commands for actions available via transferring tokens to an account
/// where this contract is deployed, using the payload in `ft_transfer_call`.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum FtMessage {
    /// Represents a request to create a new jar for a corresponding product.
    Stake(StakeMessage),

    /// Reserved for internal service use; will be removed shortly after release.
    Migrate(Vec<CeFiJar>),

    /// Represents a request to refill (top up) an existing jar using its `JarIndex`.
    TopUp(JarIndex),
}

/// The `StakeMessage` struct represents a request to create a new jar for a corresponding product.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StakeMessage {
    /// Data of the `JarTicket` required for validating the request and specifying the product.
    ticket: JarTicket,

    /// An optional ed25519 signature used to verify the authenticity of the request.
    signature: Option<Base64VecU8>,

    /// An optional account ID representing the intended owner of the created jar.
    receiver_id: Option<AccountId>,
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.assert_from_ft_contract();

        let ft_message: FtMessage = serde_json::from_str(&msg)
            .expect("Unable to deserialize msg");

        match ft_message {
            FtMessage::Stake(message) => {
                let receiver_id = message.receiver_id.unwrap_or_else(|| sender_id.clone());
                self.create_jar(
                    receiver_id,
                    message.ticket,
                    amount,
                    message.signature,
                );
            }
            FtMessage::Migrate(jars) => {
                self.migrate_jars(jars, amount);
            }
            FtMessage::TopUp(jar_index) => {
                self.top_up(jar_index, amount);
            }
        }

        PromiseOrValue::Value(0.into())
    }
}

#[cfg(test)]
mod tests {
    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::json_types::{U128, U64};
    use near_sdk::serde_json::json;
    use near_sdk::test_utils::accounts;

    use crate::common::tests::Context;
    use crate::jar::api::JarApi;
    use crate::jar::model::JarTicket;
    use crate::product::api::ProductApi;
    use crate::product::tests::{get_register_flexible_product_command, get_register_product_command, get_register_refillable_product_command, get_register_restakable_product_command};

    #[test]
    fn transfer_with_create_jar_message() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_product_command()),
        );

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": "product",
                    "valid_until": "0",
                }
            }
        });

        context.switch_account_to_ft_contract_account();
        context.contract.ft_on_transfer(
            alice,
            U128(1_000_000),
            msg.to_string(),
        );

        let jar = context.contract.get_jar(0);
        assert_eq!(jar.index, 0);
    }

    #[test]
    fn transfer_with_top_up_message_for_refillable_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_refillable_product_command()),
        );

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_register_refillable_product_command().id,
                valid_until: U64(0),
            },
            U128(100),
            None,
        );

        let msg = json!({
            "type": "top_up",
            "data": 0,
        });

        context.switch_account_to_ft_contract_account();
        context.contract.ft_on_transfer(
            alice,
            U128(100),
            msg.to_string(),
        );

        let jar = context.contract.get_jar(0);
        assert_eq!(200, jar.principal.0);
    }

    #[test]
    #[should_panic(expected = "The product doesn't allow top-ups")]
    fn transfer_with_top_up_message_for_not_refillable_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_product_command()),
        );

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_register_product_command().id,
                valid_until: U64(0),
            },
            U128(100),
            None,
        );

        let msg = json!({
            "type": "top_up",
            "data": 0,
        });

        context.switch_account_to_ft_contract_account();
        context.contract.ft_on_transfer(
            alice,
            U128(100),
            msg.to_string(),
        );
    }

    #[test]
    fn transfer_with_top_up_message_for_flexible_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_flexible_product_command()),
        );

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_register_flexible_product_command().id,
                valid_until: U64(0),
            },
            U128(100),
            None,
        );

        let msg = json!({
            "type": "top_up",
            "data": 0,
        });

        context.switch_account_to_ft_contract_account();
        context.contract.ft_on_transfer(
            alice,
            U128(100),
            msg.to_string(),
        );

        let jar = context.contract.get_jar(0);
        assert_eq!(200, jar.principal.0);
    }

    #[test]
    fn transfer_with_migration_message() {
        let alice = accounts(0);
        let bob = accounts(1);
        let admin = accounts(2);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_product_command()),
        );
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_restakable_product_command()),
        );

        let msg = json!({
            "type": "migrate",
            "data": [
                {
                    "id": "old_1",
                    "account_id": "alice",
                    "product_id": "product",
                    "principal": "100",
                    "created_at": "0",
                },
                {
                    "id": "old_2",
                    "account_id": "bob",
                    "product_id": "product_restakable",
                    "principal": "200",
                    "created_at": "0",
                },
            ]
        });

        context.switch_account_to_ft_contract_account();
        context.contract.ft_on_transfer(
            alice.clone(),
            U128(300),
            msg.to_string(),
        );

        let alice_jars = context.contract.get_jars_for_account(alice.clone());
        assert_eq!(alice_jars.len(), 1);
        assert_eq!(alice_jars.first().unwrap().principal.0, 100);

        let bob_jars = context.contract.get_jars_for_account(bob.clone());
        assert_eq!(bob_jars.len(), 1);
        assert_eq!(bob_jars.first().unwrap().principal.0, 200);
    }

    #[test]
    #[should_panic(expected = "Unable to deserialize msg")]
    fn transfer_with_unknown_message() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account_to_ft_contract_account();
        context.contract.ft_on_transfer(
            alice.clone(),
            U128(300),
            "something".to_string(),
        );
    }

    #[test]
    #[should_panic(expected = "Can receive tokens only from token")]
    fn transfer_by_not_token_account() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&alice.clone());
        context.contract.ft_on_transfer(
            alice.clone(),
            U128(300),
            "something".to_string(),
        );
    }
}
