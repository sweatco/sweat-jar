use std::collections::HashMap;

use near_sdk::{env, env::panic_str, json_types::U128, near_bindgen, require, AccountId};
use sweat_jar_model::{
    api::JarApi,
    jar::{AggregatedInterestView, AggregatedTokenAmountView, JarId, JarIdView, JarView},
    TokenAmount, U32,
};

use crate::{
    event::{emit, EventKind, RestakeData},
    jar::model::Jar,
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

        let withdraw_jar = jar.withdrawn(&product, principal, now);
        let should_be_closed = withdraw_jar.should_be_closed(&product, now);

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
        self.account_jars
            .get(&account_id)
            .unwrap_or_else(|| panic_str(&format!("Account '{account_id}' doesn't exist")))
            .get_jar(jar_id.0)
            .into()
    }

    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jars(&account_id).iter().map(Into::into).collect()
    }

    fn get_total_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView {
        self.get_principal(
            self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect(),
            account_id,
        )
    }

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

    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        self.get_interest(
            self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect(),
            account_id,
        )
    }

    fn get_interest(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> AggregatedInterestView {
        let now = env::block_timestamp_ms();

        let mut detailed_amounts = HashMap::<JarIdView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for jar in self.account_jars_with_ids(&account_id, &jar_ids) {
            let interest = jar.get_interest(&self.get_product(&jar.product_id), now).0;

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

    fn restake(&mut self, jar_id: JarIdView) -> JarView {
        self.migrate_account_jars_if_needed(env::predecessor_account_id());
        let (old_id, jar) = self.restake_internal(jar_id);

        emit(EventKind::Restake(RestakeData {
            old_id,
            new_id: jar.id.0,
        }));

        jar
    }

    fn restake_all(&mut self) -> Vec<JarView> {
        let account_id = env::predecessor_account_id();

        self.migrate_account_jars_if_needed(account_id.clone());

        let now = env::block_timestamp_ms();

        let jars: Vec<Jar> = self
            .account_jars
            .get(&account_id)
            .unwrap_or_else(|| {
                panic_str(&format!("Jars for account {account_id} don't exist"));
            })
            .jars
            .iter()
            .filter(|j| self.can_be_restaked(j, now))
            .cloned()
            .collect();

        let mut result = vec![];

        let mut event_data = vec![];

        for jar in &jars {
            let (old_id, restaked) = self.restake_internal(jar.id.into());
            event_data.push(RestakeData {
                old_id,
                new_id: restaked.id.0,
            });
            result.push(restaked);
        }

        emit(EventKind::RestakeAll(event_data));

        result
    }

    fn unlock_jars_for_account(&mut self, account_id: AccountId) {
        self.assert_manager();

        let jars = self
            .account_jars
            .get_mut(&account_id)
            .expect("Account doesn't have jars");

        for jar in &mut jars.jars {
            jar.is_pending_withdraw = false;
        }
    }
}
