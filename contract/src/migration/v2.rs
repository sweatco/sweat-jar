use std::{cell::RefCell, collections::HashMap};

#[cfg(not(test))]
use crate::ft_interface::*;
use crate::{
    assert::assert_not_locked, internal::is_promise_success, jar::model::AccountJarsLegacy, product::model::Product,
    Contract, ContractExt, MigrationState, StorageKey,
};
use crate::{internal::assert_gas, jar::account::versioned::Account as LegacyAccount};
use near_sdk::env::log_str;
use near_sdk::serde_json::{self, json};
use near_sdk::{
    borsh::to_vec,
    collections::UnorderedMap,
    env::{self, panic_str},
    json_types::Base64VecU8,
    near,
    store::{LookupMap, LookupSet},
    AccountId, Gas, PanicOnDefault, PromiseOrValue,
};
use near_sdk::{NearToken, Promise};
use sweat_jar_model::{
    account::{v1::AccountScore, versioned::AccountVersioned, Account},
    api::MigrationToV2,
    jar::JarId,
    ProductId, ScoreRecord, TokenAmount,
};

use super::account_jars_non_versioned::AccountJarsNonVersioned;

#[near]
#[derive(PanicOnDefault)]
pub struct ContractBeforeMigration {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub last_jar_id: JarId,
    pub accounts: LookupMap<AccountId, LegacyAccount>,
    pub account_jars_non_versioned: LookupMap<AccountId, AccountJarsNonVersioned>,
    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,
    #[borsh(skip)]
    pub products_cache: RefCell<HashMap<ProductId, Product>>,
}

const TGAS_FOR_MIGRATION_TRANSFER: u64 = 100;
const TGAS_FOR_MIGRATION_CALLBACK: u64 = 10;

#[near]
impl MigrationToV2 for Contract {
    #[private]
    #[init(ignore_state)]
    fn migrate_state_to_v2_ready(new_version_account_id: AccountId) -> Self {
        let old_state: ContractBeforeMigration = env::state_read().expect("Failed to extract old contract state.");

        Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products: old_state.products,
            last_jar_id: old_state.last_jar_id,
            accounts: old_state.accounts,
            account_jars_non_versioned: old_state.account_jars_non_versioned,
            account_jars_v1: old_state.account_jars_v1,
            products_cache: old_state.products_cache,
            migration: MigrationState {
                new_version_account_id,
                migrating_accounts: LookupSet::new(StorageKey::Migration),
            },
        }
    }

    fn migrate_account(&mut self) -> PromiseOrValue<()> {
        let account_id = env::predecessor_account_id();
        log_str(&format!("migrate_account({account_id})"));

        self.assert_account_is_not_migrating(&account_id);

        let (principal, memo, msg) = self.prepare_migration_params(account_id.clone());

        assert_gas(
            Gas::from_tgas(TGAS_FOR_MIGRATION_TRANSFER + TGAS_FOR_MIGRATION_CALLBACK).as_gas(),
            || format!("migrate_account({account_id})"),
        );
        log_str(&format!(
            "transfer_account({account_id}) with gas available: {}",
            Gas::from_gas(env::prepaid_gas().as_gas() - env::used_gas().as_gas())
        ));
        self.transfer_account(&account_id, principal, memo, msg)
    }

    fn migrate_products(&mut self) -> PromiseOrValue<()> {
        self.assert_manager();

        let products: Vec<product_v2::Product> = self.products.values().map(|product| product.into()).collect();
        let products_json = serde_json::to_vec(&products).unwrap_or_else(|_| panic_str("Failed to serialize products"));

        self.transfer_products(products_json)
    }
}

#[cfg(not(test))]
impl Contract {
    fn transfer_account(
        &mut self,
        account_id: &AccountId,
        principal: TokenAmount,
        memo: String,
        msg: String,
    ) -> PromiseOrValue<()> {
        self.ft_contract()
            .ft_transfer_call(
                &self.migration.new_version_account_id,
                principal,
                memo.as_str(),
                msg.as_str(),
                TGAS_FOR_MIGRATION_TRANSFER,
            )
            .then(
                Self::ext(env::current_account_id())
                    .after_account_transferred(account_id.clone())
                    .into(),
            )
            .into()
    }

    fn transfer_products(&mut self, products_json: Vec<u8>) -> PromiseOrValue<()> {
        Promise::new(self.migration.new_version_account_id.clone())
            .function_call(
                "migrate_products".to_string(),
                products_json,
                NearToken::from_yoctonear(0),
                Gas::from_tgas(TGAS_FOR_MIGRATION_TRANSFER),
            )
            .into()
    }
}

#[cfg(test)]
impl Contract {
    fn transfer_account(
        &mut self,
        account_id: &AccountId,
        _principal: TokenAmount,
        _memo: String,
        _msg: String,
    ) -> PromiseOrValue<()> {
        self.after_account_transferred(account_id.clone())
    }

    fn transfer_products(&mut self, _products_json: Vec<u8>) -> PromiseOrValue<()> {
        PromiseOrValue::Value(())
    }
}

#[near]
impl Contract {
    #[private]
    pub fn after_account_transferred(&mut self, account_id: AccountId) -> PromiseOrValue<()> {
        env::log_str(&format!(
            "after_account_transferred({account_id}) -> {}",
            is_promise_success()
        ));

        if is_promise_success() {
            self.clear_account(&account_id);
        }

        self.unlock_account(&account_id);

        PromiseOrValue::Value(())
    }
}

impl Contract {
    fn prepare_migration_params(&mut self, account_id: AccountId) -> (TokenAmount, String, String) {
        let (account, principal) = self.map_legacy_account(account_id.clone());
        if account.jars.is_empty() {
            panic_str("Nothing to migrate");
        }
        self.lock_account(&account_id);

        let account = AccountVersioned::V1(account);
        let account_vec: Base64VecU8 = to_vec(&account)
            .unwrap_or_else(|_| panic_str("Failed to serialize account"))
            .into();
        let memo = format!("migrate {account_id}");
        let msg = json!({
                "type": "migrate",
                "data": [
                    account_id.clone(),
                    account_vec,
                ]
        })
        .to_string();

        (principal, memo, msg)
    }

    fn map_legacy_account(&self, account_id: AccountId) -> (Account, TokenAmount) {
        let now = env::block_timestamp_ms();

        let score = self
            .get_score(&account_id)
            .map_or_else(ScoreRecord::default, |score| score.claimable_score());

        let mut account = Account {
            nonce: 0,
            score: self
                .get_score(&account_id)
                .map_or_else(AccountScore::default, |value| AccountScore {
                    updated: value.updated,
                    timezone: value.timezone,
                    scores: value.scores,
                    scores_history: value.scores_history,
                }),
            ..Account::default()
        };
        let mut total_principal = 0;

        let jars = self.account_jars(&account_id);
        for jar in jars.iter() {
            assert_not_locked(jar);

            let updated_jar = account.deposit(&jar.product_id, jar.principal, jar.created_at.into());
            let (interest, remainder) = jar.get_interest(&score, &self.get_product(&jar.product_id), now);
            updated_jar.add_to_cache(now, interest, remainder);

            if !account.is_penalty_applied {
                account.is_penalty_applied = jar.is_penalty_applied;
            }

            if jar.id > account.nonce {
                account.nonce = jar.id;
            }

            total_principal += jar.principal;
        }

        (account, total_principal)
    }

    fn lock_account(&mut self, account_id: &AccountId) {
        self.migration.migrating_accounts.insert(account_id.clone());
    }

    fn unlock_account(&mut self, account_id: &AccountId) {
        self.migration.migrating_accounts.remove(account_id);
    }

    fn clear_account(&mut self, account_id: &AccountId) {
        self.accounts.remove(account_id);
        self.account_jars_v1.remove(account_id);
        self.account_jars_non_versioned.remove(account_id);
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::test_env::alice;

    use crate::{common::tests::Context, jar::model::Jar, test_utils::admin};

    use super::*;

    #[test]
    fn test_prepare_migration_params() {
        let admin = admin();
        let alice = alice();

        let product_1 = Product {
            id: "product_1".to_string(),
            ..Product::new()
        };
        let product_2 = Product {
            id: "product_2".to_string(),
            ..Product::new()
        };
        let product_3 = Product {
            id: "product_3".to_string(),
            ..Product::new()
        };
        let product_4 = Product {
            id: "product_4".to_string(),
            ..Product::new()
        };

        let mut context = Context::new(admin.clone()).with_products(&[
            product_1.clone(),
            product_2.clone(),
            product_3.clone(),
            product_4.clone(),
        ]);

        context
            .contract()
            .create_jars(alice.clone(), "product_1".to_string(), 3 * 10u128.pow(18), 450);
        context
            .contract()
            .create_jars(alice.clone(), "product_2".to_string(), 5 * 10u128.pow(18), 50);
        context
            .contract()
            .create_jars(alice.clone(), "product_3".to_string(), 2 * 10u128.pow(18), 10);
        context
            .contract()
            .create_jars(alice.clone(), "product_4".to_string(), 1 * 10u128.pow(18), 10);

        context.switch_account(alice.clone());
        let (principal, memo, msg) = context.contract().prepare_migration_params(alice.clone());
        println!("principal: {principal}");
        println!("memo: {memo}");
        println!("msg: {msg}");
    }

    impl Contract {
        fn create_jars(
            &mut self,
            account_id: AccountId,
            product_id: ProductId,
            principal: TokenAmount,
            number_of_jars: u16,
        ) {
            let now = env::block_timestamp_ms();

            for _ in 0..number_of_jars {
                let id = self.increment_and_get_last_jar_id();
                let jar = Jar::create(id, account_id.clone(), product_id.clone(), principal, now);

                self.add_new_jar(&account_id, jar);
            }
        }
    }
}

mod product_v2 {
    use crate::product::model::{Apy, Cap, Product as ProductLegacy, Terms as TermsLegacy, WithdrawalFee};
    use near_sdk::{
        json_types::{Base64VecU8, U64},
        near,
    };
    use sweat_jar_model::{ProductId, Score};

    #[near(serializers=[borsh, json])]
    #[derive(Clone, Debug)]
    pub struct Product {
        pub id: ProductId,
        pub cap: Cap,
        pub terms: Terms,
        pub withdrawal_fee: Option<WithdrawalFee>,
        pub public_key: Option<Base64VecU8>,
        pub is_enabled: bool,
    }

    #[near(serializers=[borsh, json])]
    #[derive(Clone, Debug, PartialEq)]
    #[serde(tag = "type", content = "data", rename_all = "snake_case")]
    pub enum Terms {
        Fixed(FixedProductTerms),
        Flexible(FlexibleProductTerms),
        ScoreBased(ScoreBasedProductTerms),
    }

    #[near(serializers=[borsh, json])]
    #[derive(Clone, Debug, PartialEq)]
    pub struct FixedProductTerms {
        pub lockup_term: U64,
        pub apy: Apy,
    }

    #[near(serializers=[borsh, json])]
    #[derive(Clone, Debug, PartialEq)]
    pub struct FlexibleProductTerms {
        pub apy: Apy,
    }

    #[near(serializers=[borsh, json])]
    #[derive(Clone, Debug, PartialEq)]
    pub struct ScoreBasedProductTerms {
        pub score_cap: Score,
        pub lockup_term: U64,
    }

    impl From<ProductLegacy> for Product {
        fn from(value: ProductLegacy) -> Self {
            let terms: Terms = match value.terms {
                TermsLegacy::Fixed(terms) => {
                    if value.score_cap > 0 {
                        Terms::ScoreBased(ScoreBasedProductTerms {
                            lockup_term: terms.lockup_term.into(),

                            score_cap: value.score_cap,
                        })
                    } else {
                        Terms::Fixed(FixedProductTerms {
                            lockup_term: terms.lockup_term.into(),
                            apy: value.apy,
                        })
                    }
                }

                TermsLegacy::Flexible => Terms::Flexible(FlexibleProductTerms { apy: value.apy }),
            };

            Self {
                id: value.id,
                cap: value.cap,
                terms,
                withdrawal_fee: value.withdrawal_fee,
                public_key: value.public_key.map(Into::into),
                is_enabled: value.is_enabled,
            }
        }
    }
}
