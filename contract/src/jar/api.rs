use near_sdk::{AccountId, env, near_bindgen, require};
use near_sdk::json_types::U128;

use crate::*;
use crate::event::{emit, EventKind, RestakeData};
use crate::jar::view::JarView;

pub trait JarApi {
    fn restake(&mut self, jar_index: JarIndex) -> JarView;

    fn get_jar(&self, jar_index: JarIndex) -> JarView;
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView>;

    fn get_total_principal(&self, account_id: AccountId) -> U128;
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> U128;

    fn get_total_interest(&self, account_id: AccountId) -> U128;
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> U128;
}

#[near_bindgen]
impl JarApi for Contract {
    fn restake(&mut self, jar_index: JarIndex) -> JarView {
        let jar = self.get_jar_internal(jar_index);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);

        let product = self.get_product(&jar.product_id);

        require!(product.is_restakable, "The product doesn't support restaking");

        let now = env::block_timestamp_ms();
        require!(jar.is_mature(&product, now), "The jar is not mature yet");

        let index = self.jars.len() as JarIndex;
        let new_jar = Jar::create(index, jar.account_id.clone(), jar.product_id.clone(), jar.principal, now);
        let withdraw_jar = jar.withdrawn(&product, jar.principal, now);

        self.save_jar(&account_id, &withdraw_jar);
        self.save_jar(&account_id, &new_jar);

        emit(EventKind::Restaked(RestakeData { old_index: index, new_index: new_jar.index }));

        new_jar.into()
    }

    fn get_jar(&self, index: JarIndex) -> JarView {
        self.get_jar_internal(index).into()
    }

    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jar_ids(&account_id)
            .iter()
            .map(|index| self.get_jar(*index))
            .collect()
    }

    fn get_total_principal(&self, account_id: AccountId) -> U128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_principal(jar_indices)
    }

    // TODO: tests
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> U128 {
        let result = jar_indices
            .iter()
            .map(|index| self.get_jar_internal(*index).principal)
            .sum();

        U128(result)
    }

    fn get_total_interest(&self, account_id: AccountId) -> U128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_interest(jar_indices)
    }

    // TODO: tests
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> U128 {
        let now = env::block_timestamp_ms();
        let result = jar_indices
            .iter()
            .map(|index| self.get_jar_internal(*index))
            .map(|jar| jar.get_interest(&self.get_product(&jar.product_id), now))
            .sum();

        U128(result)
    }
}