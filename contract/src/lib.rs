// TODO: 1. partial claim
// TODO: 2. broadcast events
// TODO: 3. withdrowal allowance (no need)
// TODO: 4. withdrawal notification
// TODO: 5. migration

use std::cmp;
use std::str::FromStr;

use ed25519_dalek::{PublicKey, Signature};
use external::{ext_self, GAS_FOR_AFTER_TRANSFER};
use ft_interface::{FungibleTokenContract, FungibleTokenInterface};
use jar::{Jar, JarIndex};
use near_sdk::borsh::maybestd::collections::HashSet;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet, Vector};
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault, Promise,
    PromiseOrValue,
};
use product::{Product, ProductApi, ProductId};

use crate::assert::{assert_is_not_empty, assert_ownership};

mod assert;
mod common;
mod external;
mod ft_interface;
mod ft_receiver;
mod internal;
mod jar;
mod product;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub token_account_id: AccountId,
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

pub trait ContractApi {
    fn get_principal(&self, account_id: AccountId) -> Balance;
    fn get_interest(&self, account_id: AccountId) -> Balance;
    fn withdraw(&mut self, jar_id: JarIndex) -> PromiseOrValue<Balance>;
    fn claim_total(&mut self) -> PromiseOrValue<Balance>;
    fn claim_jars(
        &mut self,
        jar_indices: Vec<JarIndex>,
        amount: Option<Balance>,
    ) -> PromiseOrValue<Balance>;
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn init(token_account_id: AccountId, admin_allowlist: Vec<AccountId>) -> Self {
        let mut admin_allowlist_set = UnorderedSet::new(StorageKey::Administrators);
        admin_allowlist_set.extend(admin_allowlist.clone().into_iter().map(|item| item.into()));

        Self {
            token_account_id,
            admin_allowlist: admin_allowlist_set,
            products: UnorderedMap::new(StorageKey::Products),
            jars: Vector::new(StorageKey::Jars),
            account_jars: LookupMap::new(StorageKey::AccountJars),
        }
    }

    pub fn is_authorized_for_product(
        &self,
        account_id: AccountId,
        product_id: ProductId,
        signature: Option<String>,
    ) -> bool {
        let product = self.get_product(&product_id);

        if let Some(pk) = product.public_key {
            let signature = match signature {
                Some(ref s) => Signature::from_str(s).expect("Invalid signature"),
                None => panic!("Signature is required for private products"),
            };

            PublicKey::from_bytes(pk.clone().as_slice())
                .expect("Public key is invalid")
                .verify_strict(account_id.as_bytes(), &signature)
                .map_or(false, |_| true)
        } else {
            true
        }
    }

    #[private]
    pub fn create_jar(
        &mut self,
        account_id: AccountId,
        product_id: ProductId,
        amount: Balance,
    ) -> Jar {
        self.assert_product_exists(&product_id);

        let index = self.jars.len() as JarIndex;
        let now = env::block_timestamp_ms();
        let jar = Jar::create(index, account_id.clone(), product_id, amount, now);

        self.save_jar(&account_id, &jar);

        return jar;
    }

    #[private]
    fn after_transfer_call(jars_before_transfer: Vec<Jar>) -> Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
            .after_transfer(jars_before_transfer)
    }
}

impl ContractApi for Contract {
    fn get_principal(&self, account_id: AccountId) -> Balance {
        let jar_ids = self.account_jar_ids(&account_id).clone();

        jar_ids
            .iter()
            .map(|index| self.get_jar(*index))
            .fold(0, |acc, jar| acc + jar.principal)
    }

    fn get_interest(&self, account_id: AccountId) -> Balance {
        let now = env::block_timestamp_ms();
        let jar_ids = self.account_jar_ids(&account_id).clone();

        jar_ids
            .iter()
            .map(|index| self.get_jar(*index))
            .map(|jar| jar.get_interest(&self.get_product(&jar.product_id), now))
            .fold(0, |acc, interest| acc + interest)
    }

    fn withdraw(&mut self, jar_index: JarIndex) -> PromiseOrValue<Balance> {
        let jar = self.get_jar(jar_index);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);
        assert_is_not_empty(&jar);

        FungibleTokenContract::new(self.token_account_id.clone())
            .transfer(
                account_id.clone(),
                jar.principal.clone(),
                Self::after_transfer_call(vec![jar]),
            )
            .into()
    }

    fn claim_total(&mut self) -> PromiseOrValue<Balance> {
        let account_id = env::predecessor_account_id().clone();
        let jar_indices = self.account_jar_ids(&account_id).clone();

        self.claim_jars(jar_indices.into_iter().collect(), None)
    }

    fn claim_jars(
        &mut self,
        jar_indices: Vec<JarIndex>,
        amount: Option<Balance>,
    ) -> PromiseOrValue<Balance> {
        let account_id = env::predecessor_account_id().clone();
        let now = env::block_timestamp_ms();

        let get_interest_to_claim: Box<dyn Fn(Balance, Balance) -> Balance> = match amount {
            Some(ref a) => Box::new(|available, total| cmp::min(available, a.clone() - total)),
            None => Box::new(|available, _| available),
        };

        let jar_ids_iter = jar_indices.iter();
        let unlocked_jars: Vec<Jar> = jar_ids_iter
            .map(|index| self.get_jar(*index))
            .filter(|jar| !jar.is_pending_withdraw)
            .filter(|jar| jar.account_id == account_id)
            .collect();

        let mut total_interest_to_claim: Balance = 0;

        for jar in unlocked_jars.clone() {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(&product, now);
            let interest_to_claim =
                get_interest_to_claim(available_interest, total_interest_to_claim);

            let updated_jar = jar
                .claimed(available_interest, interest_to_claim, now)
                .locked();
            self.jars.replace(jar.index, &updated_jar);

            total_interest_to_claim += interest_to_claim;
        }

        if total_interest_to_claim > 0 {
            FungibleTokenContract::new(self.token_account_id.clone())
                .transfer(
                    account_id.clone(),
                    total_interest_to_claim.clone(),
                    Self::after_transfer_call(unlocked_jars.clone()),
                )
                .into()
        } else {
            PromiseOrValue::Value(0)
        }
    }
}

#[near_bindgen]
impl ProductApi for Contract {
    fn register_product(&mut self, product: Product) {
        self.assert_admin();

        self.products.insert(&product.id, &product);
    }

    fn get_products(&self) -> Vec<Product> {
        self.products.values_as_vector().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{
        test_utils::{accounts, VMContextBuilder},
        testing_env,
    };

    use crate::common::UDecimal;

    use super::*;

    fn get_product() -> Product {
        Product {
            id: "product".to_string(),
            lockup_term: 365 * 60 * 60 * 1000 * 1000,
            maturity_term: Some(365 * 60 * 60 * 1000 * 1000),
            notice_term: None,
            is_refillable: false,
            apy: UDecimal {
                significand: 1,
                exponent: 1,
            },
            cap: 100,
            is_restakable: false,
            withdrawal_fee: None,
            public_key: None,
        }
    }

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id.clone())
            .block_timestamp(0);

        builder
    }

    #[test]
    fn add_product_to_list_by_admin() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        contract.register_product(get_product());

        let products = contract.get_products();
        assert_eq!(products.len(), 1);
        assert_eq!(products.first().unwrap().id, "product".to_string());
    }

    #[test]
    #[should_panic(expected = "Can be performed only by admin")]
    fn add_product_to_list_by_not_admin() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(1)],
        );

        contract.register_product(get_product());
    }

    #[test]
    #[should_panic(expected = "Account alice doesn't have jars")]
    fn get_principle_with_no_jars() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(1)],
        );

        contract.get_principal(accounts(0));
    }

    #[test]
    fn get_principal_with_single_jar() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = get_product();

        contract.register_product(product.clone());
        contract.create_jar(accounts(1), product.clone().id, 100);

        testing_env!(get_context(accounts(1)).build());

        let principal = contract.get_principal(accounts(1));
        assert_eq!(principal, 100);
    }

    #[test]
    fn get_principal_with_multiple_jars() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = get_product();

        contract.register_product(product.clone());
        contract.create_jar(accounts(1), product.clone().id, 100);
        contract.create_jar(accounts(1), product.clone().id, 200);
        contract.create_jar(accounts(1), product.clone().id, 400);

        testing_env!(get_context(accounts(1)).build());

        let principal = contract.get_principal(accounts(1));
        assert_eq!(principal, 700);
    }

    #[test]
    #[should_panic(expected = "Account alice doesn't have jars")]
    fn get_total_interest_with_no_jars() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        contract.get_interest(accounts(0));
    }

    #[test]
    fn get_total_interest_with_single_jar_after_half_term() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = get_product();

        contract.register_product(product.clone());
        contract.create_jar(accounts(1), product.clone().id, 100);

        testing_env!(get_context(accounts(1))
            .block_timestamp(183 * 24 * 60 * 60 * u64::pow(10, 9))
            .build());

        let interest = contract.get_interest(accounts(1));
        assert_eq!(interest, 5);
    }

    #[test]
    fn get_total_interest_with_single_jar_on_maturity() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = get_product();

        contract.register_product(product.clone());
        contract.create_jar(accounts(1), product.clone().id, 100);

        testing_env!(get_context(accounts(1))
            .block_timestamp(366 * 24 * 60 * 60 * u64::pow(10, 9))
            .build());

        let interest = contract.get_interest(accounts(1));
        assert_eq!(interest, 10);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_maturity() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = get_product();

        contract.register_product(product.clone());
        contract.create_jar(accounts(1), product.clone().id, 100);

        testing_env!(get_context(accounts(1))
            .block_timestamp(400 * 24 * 60 * 60 * u64::pow(10, 9))
            .build());

        let interest = contract.get_interest(accounts(1));
        assert_eq!(interest, 10);
    }

    #[test]
    fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = get_product();

        contract.register_product(product.clone());
        contract.create_jar(accounts(1), product.clone().id, 100);

        testing_env!(get_context(accounts(1))
            .block_timestamp(183 * 24 * 60 * 60 * u64::pow(10, 9))
            .build());

        contract.claim_total();

        testing_env!(get_context(accounts(1))
            .block_timestamp(366 * 24 * 60 * 60 * u64::pow(10, 9))
            .build());

        let interest = contract.get_interest(accounts(1));
        assert_eq!(interest, 5);
    }

    #[test]
    fn check_authorization_for_public_product() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = get_product();
        contract.register_product(product.clone());

        let result = contract.is_authorized_for_product(accounts(0), product.id, None);
        assert_eq!(result, true);
    }

    #[test]
    #[should_panic(expected = "Signature is required for private products")]
    fn check_authorization_for_private_product_without_signature() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = Product {
            public_key: Some(b"signature".to_vec()),
            ..get_product()
        };
        contract.register_product(product.clone());

        contract.is_authorized_for_product(accounts(0), product.id, None);
    }

    #[test]
    fn check_authorization_for_private_product_with_correct_signature() {
        let context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::init(
            AccountId::new_unchecked("token".to_string()),
            vec![accounts(0)],
        );

        let product = Product {
            public_key: Some(vec![
                26, 19, 155, 89, 46, 117, 31, 171, 221, 114, 253, 247, 67, 65, 59, 77, 221, 88, 57,
                24, 102, 211, 115, 9, 238, 50, 221, 246, 161, 94, 210, 116,
            ]),
            ..get_product()
        };
        contract.register_product(product.clone());

        let result = contract.is_authorized_for_product(
            accounts(0),
            product.id,
            Some("A1CCD226C53E2C445D59B8FC2E078F39DC58B7D9F7C8D6DF45002A7FD700C3FB8569B3F7C85E5FD4B0679CD8261ACF59AFC2A68DE5735CC3221B2A9D29CEF908".to_string()),
        );
        assert_eq!(result, true);
    }

    //    #[test]
    //    fn get_half_of_interest_when_claim_on_half_term() {
    //        let context = get_context(accounts(0));
    //        testing_env!(context.build());
    //        let mut contract = Contract::init(
    //            AccountId::new_unchecked("token".to_string()),
    //            vec![accounts(0)],
    //        );
    //
    //        let product = get_product();
    //
    //        contract.register_product(product.clone());
    //        contract.create_jar(accounts(1), product.clone().id, 100);
    //
    //        testing_env!(get_context(accounts(1))
    //            .block_timestamp(183 * 24 * 60 * 60 * u64::pow(10, 9))
    //            .build());
    //
    //        let claim_promise = contract.claim();
    //
    //        if let PromiseOrValue::Promise(promise) = contract.claim() {
    //        }
    //        let interest = contract.claim();
    //
    //        assert_eq!(interest, 5);
    //    }
    //
    //    #[test]
    //    fn get_total_interest_when_claim_on_maturity() {
    //        let context = get_context(accounts(0));
    //        testing_env!(context.build());
    //        let mut contract = Contract::init(
    //            AccountId::new_unchecked("token".to_string()),
    //            vec![accounts(0)],
    //        );
    //
    //        let product = get_product();
    //
    //        contract.register_product(product.clone());
    //        contract.create_jar(accounts(1), product.clone().id, 100);
    //
    //        testing_env!(get_context(accounts(1))
    //            .block_timestamp(366 * 24 * 60 * 60 * u64::pow(10, 9))
    //            .build());
    //
    //        let interest = contract.claim();
    //
    //        assert_eq!(interest, 10);
    //    }
}
