use std::collections::HashMap;

use near_sdk::{
    borsh::to_vec, env, env::panic_str, json_types::Base64VecU8, near, serde_json, AccountId, PromiseOrValue,
};
use sweat_jar_model::{
    account::{v1::AccountScore, versioned::AccountVersioned, Account},
    api::MigrationToV2,
    ProductId, ScoreRecord, TokenAmount,
};

#[cfg(not(test))]
use crate::ft_interface::*;
use crate::{assert::assert_not_locked, internal::is_promise_success, Contract, ContractExt};

impl MigrationToV2 for Contract {
    fn migrate_account(&mut self) -> PromiseOrValue<()> {
        let account_id = env::predecessor_account_id();
        let (account, principal) = self.map_legacy_account(account_id.clone());
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
            .ft_transfer_call(&self.new_version_account_id, principal, memo.as_str(), msg.as_str())
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
            self.accounts.remove(&account_id);
            self.account_jars_v1.remove(&account_id);
            self.account_jars_non_versioned.remove(&account_id);
        } else {
            // TODO: reset state
        }

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
}

type MigratingAccount = (Account, HashMap<ProductId, TokenAmount>);
