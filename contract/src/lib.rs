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
        common::U32,
        jar::{api::JarApi, model::JarTicket},
        penalty::api::PenaltyApi,
        product::{
            api::*,
            command::RegisterProductCommand,
            tests::{get_product, get_register_premium_product_command, get_register_product_command},
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
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_product().id,
                valid_until: U64(0),
            },
            U128(100),
            None,
        );

        let principal = context.contract.get_total_principal(alice).total.0;
        assert_eq!(principal, 100);
    }

    #[test]
    fn get_principal_with_multiple_jars() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());
        context.switch_account(&admin);

        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let product = get_product();
        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.clone().id,
                valid_until: U64(0),
            },
            U128(100),
            None,
        );
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.clone().id,
                valid_until: U64(0),
            },
            U128(200),
            None,
        );
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: U64(0),
            },
            U128(400),
            None,
        );

        let principal = context.contract.get_total_principal(alice).total.0;
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
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_product().id,
                valid_until: U64(0),
            },
            U128(100_000_000),
            None,
        );

        context.set_block_timestamp_in_minutes(30);

        let interest = context.contract.get_total_interest(alice).total.0;
        assert_eq!(interest, 684);
    }

    #[test]
    fn get_total_interest_with_single_jar_on_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_product().id,
                valid_until: U64(0),
            },
            U128(100_000_000),
            None,
        );

        context.set_block_timestamp_in_days(365);

        let interest = context.contract.get_total_interest(alice).total.0;
        assert_eq!(interest, 12_000_000);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_product().id,
                valid_until: U64(0),
            },
            U128(100_000_000),
            None,
        );

        context.set_block_timestamp_in_days(400);

        let interest = context.contract.get_total_interest(alice).total.0;
        assert_eq!(interest, 12_000_000);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_product().id,
                valid_until: U64(0),
            },
            U128(100_000_000),
            None,
        );

        context.set_block_timestamp_in_days(182);

        let mut interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 5_983_561);

        context.switch_account(&alice);
        context.contract.claim_total();

        context.set_block_timestamp_in_days(365);

        interest = context.contract.get_total_interest(alice.clone()).total.0;
        assert_eq!(interest, 6_016_438);
    }

    #[test]
    fn get_total_interest_for_premium_with_penalty_after_half_term() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        fn get_product() -> RegisterProductCommand {
            // secret: [229, 112, 214, 47, 42, 153, 159, 206, 188, 235, 183, 190, 130, 112, 135, 229, 160, 73, 104, 18, 187, 114, 157, 171, 144, 241, 252, 130, 97, 221, 92, 185]
            // pk: [172, 10, 143, 66, 139, 118, 109, 28, 106, 47, 25, 194, 177, 91, 10, 125, 59, 248, 197, 165, 106, 229, 226, 198, 182, 194, 120, 168, 153, 255, 206, 112]
            get_register_premium_product_command(Some(Base64VecU8(vec![
                172, 10, 143, 66, 139, 118, 109, 28, 106, 47, 25, 194, 177, 91, 10, 125, 59, 248, 197, 165, 106, 229,
                226, 198, 182, 194, 120, 168, 153, 255, 206, 112,
            ])))
        }

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| context.contract.register_product(get_product()));

        let product_id = get_product().id;
        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id,
                valid_until: U64(4_848_379_977),
            },
            U128(100_000_000),
            Some(Base64VecU8(vec![
                221, 123, 23, 222, 212, 222, 238, 203, 202, 132, 132, 32, 21, 255, 140, 108, 93, 78, 140, 19, 235, 203,
                31, 65, 246, 152, 160, 248, 135, 19, 152, 201, 202, 196, 131, 233, 138, 42, 240, 231, 40, 39, 177, 88,
                214, 51, 148, 56, 60, 125, 224, 162, 60, 93, 254, 231, 218, 90, 140, 68, 146, 181, 183, 11,
            ])),
        );

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
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_product().id,
                valid_until: U64(0),
            },
            U128(100_000_000),
            None,
        );

        context.set_block_timestamp_in_days(400);

        context.switch_account(&alice);
        context.contract.withdraw(U32(0), None);

        let interest = context.contract.get_total_interest(alice);
        assert_eq!(12_000_000, interest.total.0);
    }
}
