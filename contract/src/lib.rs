use ed25519_dalek::{PublicKey, Signature};
use near_sdk::{
    assert_one_yocto,
    borsh::{self, maybestd::collections::HashSet, BorshDeserialize, BorshSerialize},
    env,
    json_types::Base64VecU8,
    near_bindgen,
    store::{LookupMap, UnorderedMap, Vector},
    AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise,
};
use near_self_update::SelfUpdate;
use product::model::{Apy, Product, ProductId};

use crate::{
    assert::{assert_is_not_closed, assert_ownership},
    jar::model::{Jar, JarIndex, JarState},
};

mod assert;
mod claim;
mod common;
mod event;
mod ft_interface;
mod ft_receiver;
mod internal;
mod jar;
mod migration;
mod penalty;
mod product;
mod withdraw;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, SelfUpdate)]
/// The `Contract` struct represents the state of the smart contract managing fungible token deposit jars.
pub struct Contract {
    /// The account ID of the fungible token contract (NEP-141) that this jars contract interacts with.
    pub token_account_id: AccountId,

    /// The account ID where fees for applicable operations are directed.
    pub fee_account_id: AccountId,

    /// The account ID authorized to perform sensitive operations on the contract.
    pub manager: AccountId,

    /// A collection of products, each representing terms for specific deposit jars.
    pub products: UnorderedMap<ProductId, Product>,

    /// A vector containing information about all deposit jars.
    pub jars: Vector<Jar>,

    /// A lookup map that associates account IDs with sets of jar indexes owned by each account.
    pub account_jars: LookupMap<AccountId, HashSet<JarIndex>>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Products,
    Jars,
    AccountJars,
}

#[near_bindgen]
impl Contract {
    #[init]
    #[private]
    pub fn init(token_account_id: AccountId, fee_account_id: AccountId, manager: AccountId) -> Self {
        Self {
            token_account_id,
            fee_account_id,
            manager,
            products: UnorderedMap::new(StorageKey::Products),
            jars: Vector::new(StorageKey::Jars),
            account_jars: LookupMap::new(StorageKey::AccountJars),
        }
    }
}

#[cfg(test)]
mod tests {
    use common::tests::Context;
    use near_sdk::{
        json_types::{U128, U64},
        test_utils::accounts,
    };

    use super::*;
    use crate::{
        claim::api::ClaimApi,
        common::{UDecimal, U32},
        jar::{api::JarApi, model::JarTicket},
        penalty::api::PenaltyApi,
        product::{
            api::*,
            command::RegisterProductCommand,
            helpers::MessageSigner,
            model::DowngradableApy,
            tests::{get_register_premium_product_command, get_register_product_command},
        },
        withdraw::api::WithdrawApi,
    };

    #[test]
    fn add_product_to_list_by_admin() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let products = context.contract.get_products();
        assert_eq!(products.len(), 1);
        assert_eq!(products.first().unwrap().id, "product".to_string());
    }

    #[test]
    #[should_panic(expected = "Can be performed only by admin")]
    fn add_product_to_list_by_not_admin() {
        let admin = accounts(0);
        let mut context = Context::new(admin);

        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });
    }

    #[test]
    fn get_principle_with_no_jars() {
        let alice = accounts(0);
        let admin = accounts(1);
        let context = Context::new(admin);

        let principal = context.contract.get_total_principal(alice);
        assert_eq!(principal.total.0, 0);
    }

    #[test]
    fn get_principal_with_single_jar() {
        let alice = &accounts(0);
        let admin = &accounts(1);

        let reference_product = generate_product();
        let reference_jar = Jar::generate(0, alice, &reference_product.id).principal(100);
        let context = Context::new(admin.clone())
            .with_products(&[reference_product])
            .with_jars(&[reference_jar]);

        let principal = context.contract.get_total_principal(alice.clone()).total.0;
        assert_eq!(principal, 100);
    }

    #[test]
    fn get_principal_with_multiple_jars() {
        let alice = &accounts(0);
        let admin = &accounts(1);

        let reference_product = generate_product();
        let jars = &[
            Jar::generate(0, alice, &reference_product.id).principal(100),
            Jar::generate(1, alice, &reference_product.id).principal(200),
            Jar::generate(2, alice, &reference_product.id).principal(400),
        ];

        let context = Context::new(admin.clone())
            .with_products(&[reference_product])
            .with_jars(jars);

        let principal = context.contract.get_total_principal(alice.clone()).total.0;
        assert_eq!(principal, 700);
    }

    #[test]
    fn get_total_interest_with_no_jars() {
        let alice = accounts(0);
        let admin = accounts(1);

        let context = Context::new(admin);

        let interest = context.contract.get_total_interest(alice);
        assert_eq!(interest.total.0, 0);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_30_minutes() {
        let alice = &accounts(0);
        let admin = &accounts(1);

        let reference_product = generate_product();

        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product.clone()])
            .with_jars(&[Jar::generate(0, alice, &reference_product.id).principal(100_000_000)]);

        context.set_block_timestamp_in_minutes(30);

        let interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 684);
    }

    #[test]
    fn get_total_interest_with_single_jar_on_maturity() {
        let alice = &accounts(0);
        let admin = &accounts(1);

        let reference_product = generate_product();

        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product.clone()])
            .with_jars(&[Jar::generate(0, alice, &reference_product.id).principal(100_000_000)]);

        context.set_block_timestamp_in_days(365);

        let interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 12_000_000);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_maturity() {
        let alice = &accounts(0);
        let admin = &accounts(1);

        let reference_product = generate_product();

        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product.clone()])
            .with_jars(&[Jar::generate(0, alice, &reference_product.id).principal(100_000_000)]);

        context.set_block_timestamp_in_days(400);

        let interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 12_000_000);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity() {
        let alice = &accounts(0);
        let admin = &accounts(1);

        let reference_product = generate_product();

        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product.clone()])
            .with_jars(&[Jar::generate(0, alice, &reference_product.id).principal(100_000_000)]);

        context.set_block_timestamp_in_days(182);

        let mut interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 5_983_561);

        context.switch_account(alice);
        context.contract.claim_total();

        context.set_block_timestamp_in_days(365);

        interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 6_016_438);
    }

    #[test]
    fn get_total_interest_for_premium_with_penalty_after_half_term() {
        let alice = accounts(0);
        let admin = accounts(1);

        let signer = MessageSigner::new();
        let reference_product = Product::generate("premium_product")
            .enabled(true)
            .apy(Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }))
            .public_key(signer.public_key().to_vec());
        let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(100_000_000);

        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product])
            .with_jars(&[reference_jar]);

        context.set_block_timestamp_in_days(182);

        let mut interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 9_972_602);

        context.switch_account(&admin);
        context.contract.set_penalty(0, true);

        context.set_block_timestamp_in_days(365);

        interest = context.contract.get_total_interest(alice).total.0;
        assert_eq!(interest, 10_000_000);
    }

    #[test]
    fn get_interest_after_withdraw() {
        let alice = &accounts(0);
        let admin = &accounts(1);

        let reference_product = generate_product();
        let reference_jar = &Jar::generate(0, alice, &reference_product.id).principal(100_000_000);

        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product])
            .with_jars(&[reference_jar.clone()]);

        context.set_block_timestamp_in_days(400);

        context.switch_account(alice);
        context.contract.withdraw(U32(reference_jar.index), None);

        let interest = context.contract.get_total_interest(alice.clone());
        assert_eq!(12_000_000, interest.total.0);
    }

    fn generate_product() -> Product {
        Product::generate("product")
            .enabled(true)
            .lockup_term(365 * 24 * 60 * 60 * 1000)
            .apy(Apy::Constant(UDecimal::new(12, 2)))
    }
}
