use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{json_types::U128, near, serde_json, AccountId, PromiseOrValue};
use sweat_jar_model::jar::DepositTicket;

use crate::{near_bindgen, Base64VecU8, Contract, ContractExt};

/// The `FtMessage` enum represents various commands for actions available via transferring tokens to an account
/// where this contract is deployed, using the payload in `ft_transfer_call`.
#[near(serializers=[json])]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum FtMessage {
    /// Represents a request to create a new jar for a corresponding product.
    Stake(StakeMessage),
}

/// The `StakeMessage` struct represents a request to create a new jar for a corresponding product.
#[near(serializers=[json])]
pub struct StakeMessage {
    /// Data of the `JarTicket` required for validating the request and specifying the product.
    ticket: DepositTicket,

    /// An optional ed25519 signature used to verify the authenticity of the request.
    signature: Option<Base64VecU8>,

    /// An optional account ID representing the intended owner of the created jar.
    receiver_id: Option<AccountId>,
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        self.assert_from_ft_contract();

        let ft_message: FtMessage = serde_json::from_str(&msg).expect("Unable to deserialize msg");

        match ft_message {
            FtMessage::Stake(message) => {
                let receiver_id = message.receiver_id.unwrap_or(sender_id);
                self.deposit(receiver_id, message.ticket, amount.0, &message.signature);
            }
        }

        PromiseOrValue::Value(0.into())
    }
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::{json_types::U128, serde_json::json, test_utils::test_env::alice};
    use sweat_jar_model::{
        product::{Apy, DowngradableApy, FixedProductTerms, Product, Terms},
        signer::{
            test_utils::{Base64String, MessageSigner},
            DepositMessage,
        },
        UDecimal, MS_IN_YEAR,
    };

    use crate::{common::tests::Context, test_utils::admin};

    #[test]
    fn transfer_with_create_jar_message() {
        let alice = alice();
        let admin = admin();

        let product = Product::default();
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

        let ticket_amount = 1_000_000;
        context.switch_account_to_ft_contract_account();
        context
            .contract()
            .ft_on_transfer(alice.clone(), ticket_amount.into(), msg.to_string());

        let principal = context
            .contract()
            .get_account(&alice)
            .get_jar(&product.id)
            .total_principal();
        assert_eq!(ticket_amount, principal);
    }

    #[test]
    fn transfer_with_duplicate_create_jar_message() {
        let alice = alice();
        let admin = admin();

        let (signer, product) = generate_premium_product_context();

        let mut context = Context::new(admin).with_products(&[product.clone()]);

        let ticket_amount = 1_000_000u128;
        let ticket_valid_until = 100_000_000u64;
        let message = DepositMessage::new(
            &context.owner,
            &alice,
            &product.id,
            ticket_amount,
            ticket_valid_until,
            0,
        );
        dbg!(message.to_string());
        let signature: Base64String = signer.sign(message.as_str()).into();

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": product.id,
                    "valid_until": ticket_valid_until.to_string(),
                },
                "signature": *signature,
            }
        });

        context.switch_account_to_ft_contract_account();
        context
            .contract()
            .ft_on_transfer(alice.clone(), ticket_amount.into(), msg.to_string());

        let principal = context
            .contract()
            .get_account(&alice)
            .get_jar(&product.id)
            .total_principal();
        assert_eq!(ticket_amount, principal);

        let result = catch_unwind(move || {
            context
                .contract()
                .ft_on_transfer(alice.clone(), U128(ticket_amount), msg.to_string())
        });
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "Unable to deserialize msg")]
    fn transfer_with_unknown_message() {
        let alice = alice();
        let admin = admin();

        let mut context = Context::new(admin);

        context.switch_account_to_ft_contract_account();
        context
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
        context
            .contract()
            .ft_on_transfer(alice.clone(), U128(300), "something".to_string());
    }

    fn generate_premium_product_context() -> (MessageSigner, Product) {
        let signer = MessageSigner::new();
        let product = Product::default()
            .with_public_key(signer.public_key().into())
            .with_terms(Terms::Fixed(FixedProductTerms {
                lockup_term: MS_IN_YEAR.into(),
                apy: Apy::Downgradable(DowngradableApy {
                    default: UDecimal::new(20, 2),
                    fallback: UDecimal::new(10, 2),
                }),
            }));

        (signer, product)
    }
}
