use std::collections::HashMap;

use near_sdk::{env, env::panic_str, json_types::U128, near_bindgen, require, AccountId};
use sweat_jar_model::{
    api::JarApi,
    jar::{AggregatedInterestView, AggregatedTokenAmountView, JarId, JarIdView, JarView},
    TokenAmount, JAR_BATCH_SIZE, U32,
};

use crate::{
    event::{emit, EventKind},
    jar::model::Jar,
    score::AccountScore,
    Contract, ContractExt, JarsStorage,
};

impl Contract {
    fn can_be_restaked(&self, jar: &Jar, now: u64) -> bool {
        let product = self.get_product(&jar.product_id);
        !jar.is_empty() && product.is_enabled && product.allows_restaking() && jar.is_liquidable(&product, now)
    }

    fn restake_internal(&mut self, jar_id: JarIdView) -> (JarId, JarView) {
        let jar_id = jar_id.0;
        let account_id = env::predecessor_account_id();

        let restaked_jar_id = self.increment_and_get_last_jar_id();

        let jar = self.get_jar_internal(&account_id, jar_id);

        let product = self.get_product(&jar.product_id);

        require!(product.allows_restaking(), "The product doesn't support restaking");
        require!(product.is_enabled, "The product is disabled");

        let now = env::block_timestamp_ms();
        require!(jar.is_liquidable(&product, now), "The jar is not mature yet");
        require!(!jar.is_empty(), "The jar is empty, nothing to restake");

        let principal = jar.principal;

        let new_jar = Jar::create(
            restaked_jar_id,
            jar.account_id.clone(),
            jar.product_id.clone(),
            principal,
            now,
        );

        let score = self
            .get_score(&account_id)
            .map(AccountScore::claimable_score)
            .unwrap_or_default();

        let withdraw_jar = jar.withdrawn(&score, &product, principal, now);
        let should_be_closed = withdraw_jar.should_be_closed(&score, &product, now);

        if should_be_closed {
            self.delete_jar(&withdraw_jar.account_id, withdraw_jar.id);
        } else {
            let jar_id = withdraw_jar.id;
            *self.get_jar_mut_internal(&account_id, jar_id) = withdraw_jar;
        }

        self.add_new_jar(&account_id, new_jar.clone());

        (jar_id, new_jar.into())
    }
}

#[near_bindgen]
impl JarApi for Contract {
    // TODO: restore previous version after V2 migration
    // TODO: add v2 support
    #[mutants::skip]
    fn get_jar(&self, account_id: AccountId, jar_id: JarIdView) -> JarView {
        if let Some(record) = self.account_jars_v1.get(&account_id) {
            let jar: Jar = record
                .jars
                .iter()
                .find(|jar| jar.id == jar_id.0)
                .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {} doesn't exist", jar_id.0)))
                .clone()
                .into();

            return jar.into();
        }

        if let Some(record) = self.account_jars_non_versioned.get(&account_id) {
            let jar: Jar = record
                .jars
                .iter()
                .find(|jar| jar.id == jar_id.0)
                .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {} doesn't exist", jar_id.0)))
                .clone();

            return jar.into();
        }

        self.accounts
            .get(&account_id)
            .unwrap_or_else(|| panic_str(&format!("Account '{account_id}' doesn't exist")))
            .get_jar(jar_id.0)
            .into()
    }

    // TODO: add v2 support
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jars(&account_id).iter().map(Into::into).collect()
    }

    // TODO: add v2 support
    fn get_total_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView {
        self.get_principal(
            self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect(),
            account_id,
        )
    }

    // TODO: add v2 support
    fn get_principal(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> AggregatedTokenAmountView {
        let mut detailed_amounts = HashMap::<JarIdView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for jar in self.account_jars_with_ids(&account_id, &jar_ids) {
            let id = jar.id;
            let principal = jar.principal;

            detailed_amounts.insert(U32(id), U128(principal));
            total_amount += principal;
        }

        AggregatedTokenAmountView {
            detailed: detailed_amounts,
            total: U128(total_amount),
        }
    }

    // TODO: add v2 support
    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        self.get_interest(
            self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect(),
            account_id,
        )
    }

    // TODO: add v2 support
    fn get_interest(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> AggregatedInterestView {
        let now = env::block_timestamp_ms();

        let mut detailed_amounts = HashMap::<JarIdView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        let score = self
            .get_score(&account_id)
            .map(AccountScore::claimable_score)
            .unwrap_or_default();

        for jar in self.account_jars_with_ids(&account_id, &jar_ids) {
            let product = self.get_product(&jar.product_id);

            let interest = jar.get_interest(&score, &product, now).0;

            detailed_amounts.insert(U32(jar.id), U128(interest));
            total_amount += interest;
        }

        AggregatedInterestView {
            amount: AggregatedTokenAmountView {
                detailed: detailed_amounts,
                total: U128(total_amount),
            },
            timestamp: now,
        }
    }

    // TODO: add v2 support
    fn restake(&mut self, jar_id: JarIdView) -> JarView {
        self.migrate_account_if_needed(&env::predecessor_account_id());
        let (old_id, jar) = self.restake_internal(jar_id);

        emit(EventKind::Restake((old_id, jar.id.0)));

        jar
    }

    fn restake_all(&mut self, jars: Option<Vec<JarIdView>>) -> Vec<JarView> {
        let account_id = env::predecessor_account_id();

        self.migrate_account_if_needed(&account_id);

        let now = env::block_timestamp_ms();

        let jars_filter: Option<Vec<JarId>> = jars.map(|jars| jars.into_iter().map(|j| j.0).collect());

        let mut jars: Vec<Jar> = self
            .accounts
            .get(&account_id)
            .unwrap_or_else(|| {
                panic_str(&format!("Jars for account {account_id} don't exist"));
            })
            .jars
            .iter()
            .filter(|j| self.can_be_restaked(j, now))
            .take(JAR_BATCH_SIZE)
            .cloned()
            .collect();

        if let Some(jars_filter) = jars_filter {
            jars.retain(|jar| jars_filter.contains(&jar.id));
        }

        let mut result = vec![];

        let mut event_data = vec![];

        for jar in &jars {
            let (old_id, restaked) = self.restake_internal(jar.id.into());
            event_data.push((old_id, restaked.id.0));
            result.push(restaked);
        }

        emit(EventKind::RestakeAll(event_data));

        result
    }

    // TODO: add v2 support
    fn unlock_jars_for_account(&mut self, account_id: AccountId) {
        self.assert_manager();
        self.migrate_account_if_needed(&account_id);

        let jars = self.accounts.get_mut(&account_id).expect("Account doesn't have jars");

        for jar in &mut jars.jars {
            jar.is_pending_withdraw = false;
        }
    }
}
