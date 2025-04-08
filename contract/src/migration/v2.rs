use std::{cell::RefCell, collections::HashMap};

use near_sdk::{
    borsh::to_vec, collections::UnorderedMap, env::{self, panic_str}, json_types::Base64VecU8, near, serde_json, store::{LookupMap, LookupSet}, AccountId, PanicOnDefault, PromiseOrValue
};
use sweat_jar_model::{
    account::{v1::AccountScore, versioned::AccountVersioned, Account},
    api::MigrationToV2,
    jar::JarId,
    ProductId, ScoreRecord, TokenAmount,
};
use crate::jar::account::versioned::Account as LegacyAccount;
#[cfg(not(test))]
use crate::ft_interface::*;
use crate::{
    assert::assert_not_locked, internal::is_promise_success, jar::model::AccountJarsLegacy, product::model::Product,
    Contract, ContractExt, MigrationState, StorageKey,
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

#[near]
impl MigrationToV2 for Contract {
    #[private]
    #[init(ignore_state)]
    fn migrate_state_to_v2_ready(new_version_account_id: AccountId) -> Self {
        let mut old_state: ContractBeforeMigration = env::state_read().expect("Failed to extract old contract state.");

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
        self.assert_account_is_not_migrating(&account_id);

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
        let msg =
            serde_json::to_string(&account_vec).unwrap_or_else(|_| panic_str("Unable to serialize account bytes"));

        // TODO: check amount of gas
        self.transfer_account(&account_id, principal, memo, msg)
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
            )
            .then(
                Self::ext(env::current_account_id())
                    .after_account_transferred(account_id.clone())
                    .into(),
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
}

#[near]
impl Contract {
    #[private]
    pub fn after_account_transferred(&mut self, account_id: AccountId) -> PromiseOrValue<()> {
        if is_promise_success() {
            self.clear_account(&account_id);
        }

        self.unlock_account(&account_id);

        PromiseOrValue::Value(())
    }
}

impl Contract {
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

type MigratingAccount = (Account, HashMap<ProductId, TokenAmount>);
