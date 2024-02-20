use std::collections::HashMap;

use near_sdk::require;
use sweat_jar_model::{
    jar::{JarId, JarIdView},
    ProductId,
};

use crate::{env, jar::model::Jar, AccountId, Contract, Product};

impl Contract {
    pub(crate) fn assert_manager(&self) {
        require!(
            self.manager == env::predecessor_account_id(),
            "Can be performed only by admin"
        );
    }

    pub(crate) fn assert_from_ft_contract(&self) {
        require!(
            env::predecessor_account_id() == self.token_account_id,
            format!("Can receive tokens only from {}", self.token_account_id)
        );
    }

    pub(crate) fn increment_and_get_last_jar_id(&mut self) -> JarId {
        self.last_jar_id += 1;
        self.last_jar_id
    }

    pub(crate) fn get_product(&self, product_id: &ProductId) -> &Product {
        self.products
            .get(product_id)
            .unwrap_or_else(|| env::panic_str(&format!("Product '{product_id}' doesn't exist")))
    }

    pub(crate) fn get_product_mut(&mut self, product_id: &ProductId) -> &mut Product {
        self.products
            .get_mut(product_id)
            .unwrap_or_else(|| env::panic_str(&format!("Product '{product_id}' doesn't exist")))
    }

    pub(crate) fn account_jars(&self, account_id: &AccountId) -> Vec<Jar> {
        // TODO: Remove after complete migration and return '&[Jar]`
        if let Some(record) = self.account_jars_v1.get(account_id) {
            return record.jars.iter().map(|j| j.clone().into()).collect();
        }

        self.account_jars
            .get(account_id)
            .map_or(vec![], |record| record.jars.clone())
    }

    // TODO: Restore previous version after V2 migration
    pub(crate) fn account_jars_with_ids(&self, account_id: &AccountId, ids: &[JarIdView]) -> Vec<Jar> {
        // iterates once over jars and once over ids
        let mut jars: HashMap<JarId, Jar> = self
            .account_jars(account_id)
            .into_iter()
            .map(|jar| (jar.id, jar))
            .collect();

        ids.iter()
            .map(|id| {
                jars.remove(&id.0)
                    .unwrap_or_else(|| env::panic_str(&format!("Jar with id: '{}' doesn't exist", id.0)))
            })
            .collect()
    }

    pub(crate) fn add_new_jar(&mut self, account_id: &AccountId, jar: Jar) {
        self.migrate_account_jars_if_needed(account_id.clone());
        let jars = self.account_jars.entry(account_id.clone()).or_default();
        jars.last_id = jar.id;
        jars.push(jar);
    }
}
