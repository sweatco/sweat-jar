use model::{jar::JarIdView, ProductId};
use near_sdk::require;

use crate::{env, jar::model::JarId, AccountId, Contract, Jar, Product};

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

    pub(crate) fn account_jars(&self, account_id: &AccountId) -> Vec<&Jar> {
        self.account_jars
            .get(account_id)
            .map_or(Vec::new(), |ids| ids.iter().map(|id| &self.jars[id]).collect())
    }

    pub(crate) fn jars_with_ids(&self, ids: &[JarIdView]) -> Vec<&Jar> {
        let mut result: Vec<&Jar> = vec![];

        for id in ids {
            result.push(
                self.jars
                    .get(&id.0)
                    .unwrap_or_else(|| env::panic_str(&format!("Jar with id: '{}' doesn't exist", id.0))),
            );
        }

        result
    }

    pub(crate) fn add_new_jar(&mut self, account_id: &AccountId, jar: Jar) {
        let jars = self.account_jars.entry(account_id.clone()).or_default();
        jars.last_id = jar.id;
        jars.push(jar.id);

        let existing = self.jars.insert(jar.id, jar);

        if existing.is_some() {
            env::panic_str(&format!("Jar with id: '{}' already exist", jars.last_id))
        }
    }
}
