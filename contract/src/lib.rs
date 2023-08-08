use ed25519_dalek::{PublicKey, Signature};
use near_sdk::{AccountId, assert_one_yocto, BorshStorageKey, env, Gas, near_bindgen, PanicOnDefault, Promise};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::borsh::maybestd::collections::HashSet;
use near_sdk::json_types::Base64VecU8;
use near_sdk::store::{LookupMap, UnorderedMap, UnorderedSet, Vector};

use ft_interface::FungibleTokenInterface;
use jar::{Jar, JarIndex};
use product::{Apy, Product, ProductId};

use crate::assert::{assert_is_not_closed, assert_is_not_empty, assert_ownership};
use crate::jar::{JarApi, JarState};

mod assert;
mod common;
mod external;
mod ft_interface;
mod ft_receiver;
mod internal;
mod jar;
mod product;
mod claim;
mod withdraw;
mod event;
mod migration;

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

    pub fn is_authorized_for_product(
        &self,
        account_id: &AccountId,
        product_id: &ProductId,
        signature: Option<Vec<u8>>,
    ) -> bool {
        let product = self.get_product(product_id);

        if let Some(pk) = product.public_key {
            let signature = match signature {
                Some(ref s) => Signature::from_bytes(s).expect("Invalid signature"),
                None => env::panic_str("Signature is required for private products"),
            };

            PublicKey::from_bytes(pk.as_slice())
                .expect("Public key is invalid")
                .verify_strict(account_id.as_bytes(), &signature)
                .is_ok()
        } else {
            true
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
    use near_sdk::json_types::U128;
    use near_sdk::test_utils::accounts;

    use common::tests::Context;

    use crate::claim::ClaimApi;
    use crate::jar::JarTicket;
    use crate::product::ProductApi;
    use crate::product::tests::{get_premium_product, get_product};

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
        context.contract.register_product(get_product());

        let products = context.contract.get_products();
        assert_eq!(products.len(), 1);
        assert_eq!(products.first().unwrap().id, "product".to_string());
    }

    #[test]
    #[should_panic(expected = "Can be performed only by admin")]
    fn add_product_to_list_by_not_admin() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin]);

        context.contract.register_product(get_product());
    }

    #[test]
    #[should_panic(expected = "Account alice doesn't have jars")]
    fn get_principle_with_no_jars() {
        let alice = accounts(0);
        let context = Context::new(vec![]);

        context.contract.get_total_principal(alice);
    }

    #[test]
    fn get_principal_with_single_jar() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);

        let product = get_product();
        context.contract.register_product(product.clone());

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: 0,
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

        let product = get_product();
        context.contract.register_product(product.clone());

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.clone().id,
                valid_until: 0,
            },
            U128(100),
            None,
        );
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.clone().id,
                valid_until: 0,
            },
            U128(200),
            None,
        );
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.clone().id,
                valid_until: 0,
            },
            U128(400),
            None,
        );

        let principal = context.contract.get_total_principal(alice).0;
        assert_eq!(principal, 700);
    }

    #[test]
    #[should_panic(expected = "Account alice doesn't have jars")]
    fn get_total_interest_with_no_jars() {
        let alice = accounts(0);

        let context = Context::new(vec![]);

        context.contract.get_total_interest(alice);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_30_minutes() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        let product = get_product();
        context.contract.register_product(product.clone());

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: 0,
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
        let product = get_product();
        context.contract.register_product(product.clone());

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: 0,
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
        let product = get_product();
        context.contract.register_product(product.clone());

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: 0,
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
        let product = get_product();
        context.contract.register_product(product.clone());

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: 0,
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
    fn check_authorization_for_public_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        let product = get_product();
        context.contract.register_product(product.clone());

        let result = context.contract.is_authorized_for_product(&alice, &product.id, None);
        assert!(result);
    }

    #[test]
    #[should_panic(expected = "Signature is required for private products")]
    fn check_authorization_for_private_product_without_signature() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        let product = get_premium_product();
        context.contract.register_product(product.clone());

        context.contract.is_authorized_for_product(&alice, &product.id, None);
    }

    #[test]
    fn get_total_interest_for_premium_with_penalty_after_half_term() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        let product = get_premium_product();
        context.contract.register_product(product.clone());

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: product.id,
                valid_until: 1000000,
            },
            U128(100_000_000),
            Some(
                Base64VecU8(
                    vec![
                        126, 76, 136, 40, 234, 193, 197, 143, 119, 86, 135, 170, 247, 130, 173, 154, 88, 43,
                        224, 78, 2, 2, 67, 243, 189, 28, 138, 43, 92, 93, 147, 187, 200, 62, 118, 158, 164, 108,
                        140, 154, 144, 147, 250, 112, 234, 255, 248, 213, 107, 224, 201, 147, 186, 233, 120, 56,
                        21, 160, 85, 204, 135, 240, 61, 13,
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
