use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    json_types::U128,
    require,
    serde::{Deserialize, Serialize},
    serde_json, PromiseOrValue,
};

use crate::{
    jar::model::JarTicket, migration::model::CeFiJar, near_bindgen, AccountId, Base64VecU8, Contract, ContractExt,
    JarID,
};

/// The `FtMessage` enum represents various commands for actions available via transferring tokens to an account
/// where this contract is deployed, using the payload in `ft_transfer_call`.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum FtMessage {
    /// Represents a request to create a new jar for a corresponding product.
    Stake(StakeMessage),

    /// Represents a request to create DeFi Jars from provided CeFi Jars.
    Migrate(Vec<CeFiJar>),

    /// Represents a request to refill (top up) an existing jar using its `JarIndex`.
    TopUp(JarID),
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
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        self.assert_from_ft_contract();

        let ft_message: FtMessage = serde_json::from_str(&msg).expect("Unable to deserialize msg");

        match ft_message {
            FtMessage::Stake(message) => {
                let receiver_id = message.receiver_id.unwrap_or(sender_id);
                self.create_jar(receiver_id, message.ticket, amount, message.signature);
            }
            FtMessage::Migrate(jars) => {
                require!(sender_id == self.manager, "Migration can be performed only by admin");

                self.migrate_jars(jars, amount);
            }
            FtMessage::TopUp(jar_index) => {
                self.top_up(&sender_id, jar_index, amount);
            }
        }

        PromiseOrValue::Value(0.into())
    }
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::{json_types::U128, serde_json::json, test_utils::accounts};

    use crate::{
        common::{tests::Context, u32::U32, udecimal::UDecimal},
        jar::{api::JarApi, model::Jar},
        product::{
            helpers::MessageSigner,
            model::{Apy, DowngradableApy, Product},
        },
        Contract,
    };

    #[test]
    fn transfer_with_create_jar_message() {
        let alice = accounts(0);
        let admin = accounts(1);

        let reference_product = Product::generate("test_product").enabled(true);
        let mut context = Context::new(admin).with_products(&[reference_product.clone()]);

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": reference_product.id,
                    "valid_until": "0",
                }
            }
        });

        context.switch_account_to_ft_contract_account();
        context
            .contract
            .ft_on_transfer(alice.clone(), U128(1_000_000), msg.to_string());

        let jar = context.contract.get_jar(alice, U32(1));
        assert_eq!(jar.id.0, 1);
    }

    #[test]
    fn transfer_with_duplicate_create_jar_message() {
        let alice = accounts(0);
        let admin = accounts(1);

        let (signer, reference_product) = generate_premium_product_context();

        let mut context = Context::new(admin).with_products(&[reference_product.clone()]);

        let ticket_amount = 1_000_000u128;
        let ticket_valid_until = 100_000_000u64;
        let signature = signer.sign_base64(
            Contract::get_signature_material(
                &context.owner,
                &alice,
                &reference_product.id,
                ticket_amount,
                ticket_valid_until,
                None,
            )
            .as_str(),
        );

        let msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": reference_product.id,
                    "valid_until": ticket_valid_until.to_string(),
                },
                "signature": signature,
            }
        });

        context.switch_account_to_ft_contract_account();
        context
            .contract
            .ft_on_transfer(alice.clone(), U128(ticket_amount), msg.to_string());

        let jar = context.contract.get_jar(alice.clone(), U32(1));
        assert_eq!(jar.id.0, 1);

        let result = catch_unwind(move || {
            context
                .contract
                .ft_on_transfer(alice.clone(), U128(ticket_amount), msg.to_string())
        });
        assert!(result.is_err());
    }

    #[test]
    fn transfer_with_top_up_message_for_refillable_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let reference_product = Product::generate("refillable_product")
            .enabled(true)
            .with_allows_top_up(true)
            .cap(0, 1_000_000);

        let initial_jar_principal = 100;
        let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(initial_jar_principal);

        let mut context = Context::new(admin)
            .with_products(&[reference_product])
            .with_jars(&[reference_jar.clone()]);

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();
        let top_up_amount = 700;
        context
            .contract
            .ft_on_transfer(alice.clone(), U128(top_up_amount), msg.to_string());

        let jar = context.contract.get_jar(alice, U32(0));
        assert_eq!(initial_jar_principal + top_up_amount, jar.principal.0);
    }

    #[test]
    #[should_panic(expected = "The product doesn't allow top-ups")]
    fn transfer_with_top_up_message_for_not_refillable_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let reference_product = Product::generate("not_refillable_product")
            .enabled(true)
            .with_allows_top_up(false)
            .cap(0, 1_000_000);

        let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(500);

        let mut context = Context::new(admin)
            .with_products(&[reference_product])
            .with_jars(&[reference_jar.clone()]);

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();
        context.contract.ft_on_transfer(alice, U128(100), msg.to_string());
    }

    #[test]
    fn transfer_with_top_up_message_for_flexible_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let reference_product = Product::generate("flexible_product")
            .enabled(true)
            .flexible()
            .cap(0, 1_000_000);

        let initial_jar_principal = 100_000;
        let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(initial_jar_principal);

        let mut context = Context::new(admin)
            .with_products(&[reference_product])
            .with_jars(&[reference_jar.clone()]);

        let msg = json!({
            "type": "top_up",
            "data": reference_jar.id,
        });

        context.switch_account_to_ft_contract_account();

        let top_up_amount = 1_000;
        context
            .contract
            .ft_on_transfer(alice.clone(), U128(top_up_amount), msg.to_string());

        let jar = context.contract.get_jar(alice, U32(0));
        assert_eq!(initial_jar_principal + top_up_amount, jar.principal.0);
    }

    #[test]
    fn transfer_with_migration_message() {
        let alice = accounts(0);
        let bob = accounts(1);
        let admin = accounts(2);

        let reference_product = Product::generate("product").enabled(true).cap(0, 1_000_000);
        let reference_restakable_product = Product::generate("restakable_product").enabled(true).cap(0, 1_000_000);

        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product.clone(), reference_restakable_product.clone()]);

        let amount_alice = 100;
        let amount_bob = 200;
        let msg = json!({
            "type": "migrate",
            "data": [
                {
                    "id": "cefi_product_1",
                    "account_id": alice,
                    "product_id": reference_product.id,
                    "principal": amount_alice.to_string(),
                    "created_at": "0",
                },
                {
                    "id": "cefi_product_2",
                    "account_id": bob,
                    "product_id": reference_restakable_product.id,
                    "principal": amount_bob.to_string(),
                    "created_at": "0",
                },
            ]
        });

        context.switch_account_to_ft_contract_account();
        context
            .contract
            .ft_on_transfer(admin, U128(amount_alice + amount_bob), msg.to_string());

        let alice_jars = context.contract.get_jars_for_account(alice);
        assert_eq!(alice_jars.len(), 1);
        assert_eq!(alice_jars.first().unwrap().principal.0, amount_alice);

        let bob_jars = context.contract.get_jars_for_account(bob);
        assert_eq!(bob_jars.len(), 1);
        assert_eq!(bob_jars.first().unwrap().principal.0, amount_bob);
    }

    #[test]
    #[should_panic(expected = "Migration can be performed only by admin")]
    fn transfer_with_migration_message_by_not_admin() {
        let alice = accounts(0);
        let admin = accounts(1);

        let reference_product = Product::generate("product").enabled(true).cap(0, 1_000_000);
        let reference_restakable_product = Product::generate("restakable_product").enabled(true).cap(0, 1_000_000);

        let mut context = Context::new(admin).with_products(&[reference_product.clone(), reference_restakable_product]);

        let amount_alice = 1_000;
        let msg = json!({
            "type": "migrate",
            "data": [
                {
                    "id": "cefi_product_3",
                    "account_id": alice,
                    "product_id": reference_product.id,
                    "principal": amount_alice.to_string(),
                    "created_at": "0",
                },
            ]
        });

        context.switch_account_to_ft_contract_account();
        context
            .contract
            .ft_on_transfer(alice, U128(amount_alice), msg.to_string());
    }

    #[test]
    #[should_panic(expected = "Unable to deserialize msg")]
    fn transfer_with_unknown_message() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin);

        context.switch_account_to_ft_contract_account();
        context
            .contract
            .ft_on_transfer(alice, U128(300), "something".to_string());
    }

    #[test]
    #[should_panic(expected = "Can receive tokens only from token")]
    fn transfer_by_not_token_account() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin);

        context.switch_account(&alice);
        context
            .contract
            .ft_on_transfer(alice.clone(), U128(300), "something".to_string());
    }

    fn generate_premium_product_context() -> (MessageSigner, Product) {
        let signer = MessageSigner::new();
        let reference_product = Product::generate("premium")
            .public_key(signer.public_key())
            .lockup_term(365 * 24 * 60 * 60 * 1000)
            .apy(Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }));

        (signer, reference_product)
    }
}
