use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{json_types::U128, near, serde_json, AccountId, PromiseOrValue};
use sweat_jar_model::jar::JarId;

use crate::{jar::model::JarTicket, Base64VecU8, Contract, ContractExt};

/// The `FtMessage` enum represents various commands for actions available via transferring tokens to an account
/// where this contract is deployed, using the payload in `ft_transfer_call`.
#[near(serializers=[json])]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum FtMessage {
    /// Represents a request to create a new jar for a corresponding product.
    Stake(StakeMessage),

    /// Represents a request to refill (top up) an existing jar using its `JarId`.
    TopUp(JarId),
}

/// The `StakeMessage` struct represents a request to create a new jar for a corresponding product.
#[near(serializers=[json])]
pub struct StakeMessage {
    /// Data of the `JarTicket` required for validating the request and specifying the product.
    ticket: JarTicket,

    /// An optional ed25519 signature used to verify the authenticity of the request.
    signature: Option<Base64VecU8>,

    /// An optional account ID representing the intended owner of the created jar.
    receiver_id: Option<AccountId>,
}

#[near]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        self.assert_from_ft_contract();

        let ft_message: FtMessage = serde_json::from_str(&msg).expect("Unable to deserialize msg");

        match ft_message {
            FtMessage::Stake(message) => {
                let receiver_id = message.receiver_id.unwrap_or(sender_id);
                self.create_jar(receiver_id, message.ticket, amount, message.signature);
            }
            FtMessage::TopUp(jar_id) => {
                self.top_up(&sender_id, jar_id, amount);
            }
        }

        PromiseOrValue::Value(0.into())
    }
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::{
        json_types::U128,
        serde_json::json,
        test_utils::test_env::{alice, bob},
    };
    use sweat_jar_model::{api::JarApi, UDecimal, U32};

    use crate::{
        common::tests::Context,
        jar::model::Jar,
        product::{
            helpers::MessageSigner,
            model::{Apy, DowngradableApy, Product},
        },
        test_utils::{admin, expect_panic},
        Contract,
    };

    #[test]
    fn transfer_with_create_jar_message() {
        let alice = alice();
        let admin = admin();

        let product = Product::new();
        let mut context = Context::new(admin).with_products(&[product.clone()]);

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": product.id,
                    "valid_until": "0",
                }
            }
        });

        context.switch_account_to_ft_contract_account();
        let _ = context
            .contract()
            .ft_on_transfer(alice.clone(), U128(1_000_000), msg.to_string());

        let jar = context.contract().get_jar(alice, U32(1));
        assert_eq!(jar.id.0, 1);
    }

    #[test]
    fn transfer_with_duplicate_create_jar_message() {
        let alice = alice();
        let admin = admin();

        let (signer, product) = generate_premium_product_context();

        let mut context = Context::new(admin).with_products(&[product.clone()]);

        let ticket_amount = 1_000_000u128;
        let ticket_valid_until = 100_000_000u64;
        let signature = signer.sign_base64(
            Contract::get_signature_material(
                &context.owner,
                &alice,
                &product.id,
                ticket_amount,
                None,
                ticket_valid_until,
            )
            .as_str(),
        );

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": product.id,
                    "valid_until": ticket_valid_until.to_string(),
                },
                "signature": signature,
            }
        });

        context.switch_account_to_ft_contract_account();
        let _ = context
            .contract()
            .ft_on_transfer(alice.clone(), U128(ticket_amount), msg.to_string());

        let jar = context.contract().get_jar(alice.clone(), U32(1));
        assert_eq!(jar.id.0, 1);

        let result = catch_unwind(move || {
            context
                .contract()
                .ft_on_transfer(alice.clone(), U128(ticket_amount), msg.to_string())
        });
        assert!(result.is_err());
    }

    #[test]
    fn signature_for_step_jar() {
        let alice = alice();
        let admin = admin();

        let (signer, product) = generate_premium_product_context();

        let product = product.apy(Apy::Constant(UDecimal::default())).score_cap(20_000);

        let mut context = Context::new(admin).with_products(&[product.clone()]);

        let ticket_amount = 1_000_000u128;
        let ticket_valid_until = 100_000_000u64;
        let signature = signer.sign_base64(
            Contract::get_signature_material(
                &context.owner,
                &alice,
                &product.id,
                ticket_amount,
                None,
                ticket_valid_until,
            )
            .as_str(),
        );

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": product.id,
                    "valid_until": ticket_valid_until.to_string(),
                    "timezone": 3,
                },
                "signature": signature,
            }
        });

        context.switch_account_to_ft_contract_account();
        let _ = context
            .contract()
            .ft_on_transfer(alice.clone(), U128(ticket_amount), msg.to_string());

        let jar = context.contract().get_jar(alice.clone(), U32(1));
        assert_eq!(jar.id.0, 1);
    }

    #[test]
    fn step_jar_with_out_of_range_timezone_is_rejected() {
        let alice = alice();
        let admin = admin();

        let product = Product::new().score_cap(20_000);
        let mut context = Context::new(admin).with_products(&[product.clone()]);

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": { "product_id": product.id, "valid_until": "0", "timezone": i64::MIN + 1 }
            }
        });

        context.switch_account_to_ft_contract_account();
        expect_panic(&context, "Timezone is outside the valid UTC range", || {
            let _ = context
                .contract()
                .ft_on_transfer(alice.clone(), U128(1_000_000), msg.to_string());
        });
    }

    // A depositor may set an in-range timezone for someone else's step jar — this
    // is how oracle airdrops open jars on a user's behalf.
    #[test]
    fn third_party_can_set_step_jar_timezone() {
        let sender = alice();
        let receiver = bob();
        let admin = admin();

        let product = Product::new().score_cap(20_000);
        let mut context = Context::new(admin).with_products(&[product.clone()]);

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": { "product_id": product.id, "valid_until": "0", "timezone": 3 },
                "receiver_id": receiver,
            }
        });

        context.switch_account_to_ft_contract_account();
        let _ = context
            .contract()
            .ft_on_transfer(sender.clone(), U128(1_000_000), msg.to_string());

        assert!(context.contract().get_score(&receiver).is_some());
    }

    #[test]
    fn transfer_with_top_up_message_for_refillable_product() {
        let alice = alice();
        let admin = admin();

        let product = Product::new().with_allows_top_up(true);

        let initial_jar_principal = 100;
        let reference_jar = Jar::new(0).principal(initial_jar_principal);

        let mut context = Context::new(admin)
            .with_products(&[product])
            .with_jars(&[reference_jar.clone()]);

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();
        let top_up_amount = 700;
        let _ = context
            .contract()
            .ft_on_transfer(alice.clone(), U128(top_up_amount), msg.to_string());

        let jar = context.contract().get_jar(alice, U32(0));
        assert_eq!(initial_jar_principal + top_up_amount, jar.principal.0);
    }

    #[test]
    #[should_panic(expected = "The product doesn't allow top-ups")]
    fn transfer_with_top_up_message_for_not_refillable_product() {
        let alice = alice();
        let admin = admin();

        let product = Product::new().with_allows_top_up(false);

        let reference_jar = Jar::new(0).principal(500);

        let mut context = Context::new(admin)
            .with_products(&[product])
            .with_jars(&[reference_jar.clone()]);

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();
        let _ = context.contract().ft_on_transfer(alice, U128(100), msg.to_string());
    }

    #[test]
    fn transfer_with_top_up_message_for_flexible_product() {
        let alice = alice();
        let admin = admin();

        let product = Product::new().flexible();

        let initial_jar_principal = 100_000;
        let reference_jar = Jar::new(0).principal(initial_jar_principal);

        let mut context = Context::new(admin)
            .with_products(&[product])
            .with_jars(&[reference_jar.clone()]);

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();

        let top_up_amount = 1_000;
        let _ = context
            .contract()
            .ft_on_transfer(alice.clone(), U128(top_up_amount), msg.to_string());

        let jar = context.contract().get_jar(alice, U32(0));
        assert_eq!(initial_jar_principal + top_up_amount, jar.principal.0);
    }

    #[test]
    #[should_panic(expected = "Another operation on this Jar is in progress")]
    fn top_up_of_locked_jar_is_rejected() {
        let alice = alice();
        let admin = admin();

        let product = Product::new().with_allows_top_up(true);
        let reference_jar = Jar::new(0).principal(100).pending_withdraw();

        let mut context = Context::new(admin)
            .with_products(&[product])
            .with_jars(&[reference_jar.clone()]);

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();
        let _ = context.contract().ft_on_transfer(alice, U128(100), msg.to_string());
    }

    #[test]
    #[should_panic(expected = "Account is migrating")]
    fn top_up_while_account_is_migrating_is_rejected() {
        let alice = alice();
        let admin = admin();

        let product = Product::new().with_allows_top_up(true);
        let reference_jar = Jar::new(0).principal(100);

        let mut context = Context::new(admin)
            .with_products(&[product])
            .with_jars(&[reference_jar.clone()]);

        context.contract().migration.migrating_accounts.insert(alice.clone());

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();
        let _ = context.contract().ft_on_transfer(alice, U128(100), msg.to_string());
    }

    #[test]
    #[should_panic(expected = "Unable to deserialize msg")]
    fn transfer_with_unknown_message() {
        let alice = alice();
        let admin = admin();

        let mut context = Context::new(admin);

        context.switch_account_to_ft_contract_account();
        let _ = context
            .contract()
            .ft_on_transfer(alice, U128(300), "something".to_string());
    }

    #[test]
    #[should_panic(expected = "Can receive tokens only from token")]
    fn transfer_by_not_token_account() {
        let alice = alice();
        let admin = admin();

        let mut context = Context::new(admin);

        context.switch_account(&alice);
        let _ = context
            .contract()
            .ft_on_transfer(alice.clone(), U128(300), "something".to_string());
    }

    fn generate_premium_product_context() -> (MessageSigner, Product) {
        let signer = MessageSigner::new();
        let product = Product::new()
            .public_key(signer.public_key())
            .apy(Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }));

        (signer, product)
    }
}
