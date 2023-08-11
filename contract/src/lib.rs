use ed25519_dalek::{PublicKey, Signature};
use near_sdk::{AccountId, assert_one_yocto, BorshStorageKey, env, Gas, near_bindgen, PanicOnDefault, Promise};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::borsh::maybestd::collections::HashSet;
use near_sdk::json_types::Base64VecU8;
use near_sdk::store::{LookupMap, UnorderedMap, UnorderedSet, Vector};

use product::model::{Apy, Product, ProductId};

use crate::assert::{assert_is_not_closed, assert_is_not_empty, assert_ownership};
use crate::jar::model::{Jar, JarIndex, JarState};

mod assert;
mod common;
mod external;
mod ft_interface;
mod ft_receiver;
mod internal;
mod jar;
mod claim;
mod withdraw;
mod event;
mod migration;
mod product;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub admin_allowlist: UnorderedSet<AccountId>,

    pub products: UnorderedMap<ProductId, Product>,

    pub jars: Vector<Jar>,
    pub account_jars: LookupMap<AccountId, HashSet<JarIndex>>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Administrators,
    Products,
    Jars,
    AccountJars,
}

pub trait AuthApi {
    fn get_admin_allowlist(&self) -> Vec<AccountId>;
    fn add_admin(&mut self, account_id: AccountId);
    fn remove_admin(&mut self, account_id: AccountId);
}

pub trait PenaltyApi {
    fn set_penalty(&mut self, jar_index: JarIndex, value: bool);
}

#[near_bindgen]
impl Contract {
    pub fn time() -> u64 {
        env::block_timestamp_ms()
    }

    #[init]
    #[private]
    pub fn init(
        token_account_id: AccountId,
        fee_account_id: AccountId,
        admin_allowlist: Vec<AccountId>,
    ) -> Self {
        let mut admin_allowlist_set = UnorderedSet::new(StorageKey::Administrators);
        admin_allowlist_set.extend(admin_allowlist.into_iter());

        Self {
            token_account_id,
            fee_account_id,
            admin_allowlist: admin_allowlist_set,
            products: UnorderedMap::new(StorageKey::Products),
            jars: Vector::new(StorageKey::Jars),
            account_jars: LookupMap::new(StorageKey::AccountJars),
        }
    }
}

#[near_bindgen]
impl AuthApi for Contract {
    fn get_admin_allowlist(&self) -> Vec<AccountId> {
        self.admin_allowlist.iter().cloned().collect()
    }

    fn add_admin(&mut self, account_id: AccountId) {
        assert_one_yocto();
        self.assert_admin();

        self.admin_allowlist.insert(account_id);
    }

    fn remove_admin(&mut self, account_id: AccountId) {
        assert_one_yocto();
        self.assert_admin();

        self.admin_allowlist.remove(&account_id);
    }
}

#[near_bindgen]
impl PenaltyApi for Contract {
    //TODO: add event
    fn set_penalty(&mut self, jar_index: JarIndex, value: bool) {
        self.assert_admin();

        let jar = self.get_jar_internal(jar_index);
        let product = self.get_product(&jar.product_id);

        match product.apy {
            Apy::Downgradable(_) => {
                let updated_jar = jar.with_penalty_applied(value);
                self.jars.replace(jar.index, updated_jar);
            }
            _ => env::panic_str("Penalty is not applicable"),
        };
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::json_types::{U128, U64};
    use near_sdk::test_utils::accounts;

    use common::tests::Context;

    use crate::claim::api::ClaimApi;
    use crate::jar::api::JarApi;
    use crate::jar::model::JarTicket;
    use crate::product::api::*;
    use crate::product::tests::{get_premium_product, get_product, get_register_premium_product_command, get_register_product_command};

    use super::*;

    #[test]
    fn add_admin_by_admin() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.with_deposit_yocto(1);

        context.contract.add_admin(alice.clone());
        let admins = context.contract.get_admin_allowlist();

        assert_eq!(2, admins.len());
        assert!(admins.contains(&alice));
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn add_admin_with_no_deposit() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin]);
        context.switch_account(&alice);

        context.contract.add_admin(alice.clone());
    }

    #[test]
    #[should_panic(expected = "Can be performed only by admin")]
    fn add_admin_by_not_admin() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin]);
        context.switch_account(&alice);
        context.with_deposit_yocto(1);

        context.contract.add_admin(alice.clone());
    }

    #[test]
    fn remove_admin_by_admin() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone(), alice.clone()]);
        context.switch_account(&admin);
        context.with_deposit_yocto(1);

        context.contract.remove_admin(alice.clone());
        let admins = context.contract.get_admin_allowlist();

        assert_eq!(1, admins.len());
        assert!(!admins.contains(&alice));
    }

    #[test]
    #[should_panic(expected = "Can be performed only by admin")]
    fn remove_admin_by_not_admin() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);
        context.switch_account(&alice);
        context.with_deposit_yocto(1);

        context.contract.remove_admin(admin);
    }

    #[test]
    fn add_product_to_list_by_admin() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

        let products = context.contract.get_products();
        assert_eq!(products.len(), 1);
        assert_eq!(products.first().unwrap().id, "product".to_string());
    }

    #[test]
    #[should_panic(expected = "Can be performed only by admin")]
    fn add_product_to_list_by_not_admin() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin]);

        context.contract.register_product(get_register_product_command());
    }

    #[test]
    fn get_principle_with_no_jars() {
        let alice = accounts(0);
        let context = Context::new(vec![]);

        let principal = context.contract.get_total_principal(alice);
        assert_eq!(principal.0, 0);
    }

    #[test]
    fn get_principal_with_single_jar() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);

        context.contract.register_product(get_register_product_command());

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

        let principal = context.contract.get_total_principal(alice).0;
        assert_eq!(principal, 100);
    }

    #[test]
    fn get_principal_with_multiple_jars() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);
        context.switch_account(&admin);

        context.contract.register_product(get_register_product_command());

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

        let principal = context.contract.get_total_principal(alice).0;
        assert_eq!(principal, 700);
    }

    #[test]
    fn get_total_interest_with_no_jars() {
        let alice = accounts(0);

        let context = Context::new(vec![]);

        let interest = context.contract.get_total_interest(alice);
        assert_eq!(interest.0, 0);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_30_minutes() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

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

        let interest = context.contract.get_total_interest(alice).0;
        assert_eq!(interest, 684);
    }

    #[test]
    fn get_total_interest_with_single_jar_on_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

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

        let interest = context.contract.get_total_interest(alice).0;
        assert_eq!(interest, 12_000_000);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

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

        let interest = context.contract.get_total_interest(alice).0;
        assert_eq!(interest, 12_000_000);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

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

        let mut interest = context.contract.get_total_interest(alice.clone()).0;
        assert_eq!(interest, 5_983_561);

        context.switch_account(&alice);
        context.contract.claim_total();

        context.set_block_timestamp_in_days(365);

        interest = context.contract.get_total_interest(alice.clone()).0;
        assert_eq!(interest, 6_016_438);
    }

    #[test]
    fn get_total_interest_for_premium_with_penalty_after_half_term() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_premium_product_command());

        let product = get_premium_product();
        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: U64(100_000_000),
            },
            U128(100_000_000),
            Some(
                Base64VecU8(
                    vec![
                        106, 169, 28, 95, 190, 177, 11, 212, 73, 215, 174, 31, 143, 61, 191, 107, 132, 100, 38,
                        8, 90, 248, 246, 79, 84, 216, 122, 215, 182, 136, 134, 160, 3, 10, 118, 74, 123, 31, 91,
                        121, 192, 142, 25, 97, 54, 231, 253, 26, 239, 15, 24, 201, 110, 243, 6, 134, 246, 17,
                        148, 178, 251, 68, 57, 13,
                    ]
                )
            ),
        );

        context.set_block_timestamp_in_days(182);

        let mut interest = context.contract.get_total_interest(alice.clone()).0;
        assert_eq!(interest, 9_972_602);

        context.switch_account(&admin);
        context.contract.set_penalty(0, true);

        context.set_block_timestamp_in_days(365);

        interest = context.contract.get_total_interest(alice).0;
        assert_eq!(interest, 10_000_000);
    }
}
