use near_sdk::require;

use crate::{env, AccountId, Contract, Jar, JarIndex, Product, ProductId};

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

    pub(crate) fn get_product(&self, product_id: &ProductId) -> &Product {
        self.products
            .get(product_id)
            .unwrap_or_else(|| env::panic_str(&format!("Product {product_id} doesn't exist")))
    }

    pub(crate) fn get_product_mut(&mut self, product_id: &ProductId) -> &mut Product {
        self.products
            .get_mut(product_id)
            .unwrap_or_else(|| env::panic_str(&format!("Product {product_id} doesn't exist")))
    }

    pub(crate) fn account_jars(&self, account_id: &AccountId) -> Vec<Jar> {
        self.account_jars
            .get(account_id)
            .map_or_else(Vec::new, |items| items.jars.iter().cloned().collect())
    }

    pub(crate) fn save_jar(&mut self, account_id: &AccountId, jar: Jar) {
        let jar_index = jar.index;
        self.insert_or_update_jar(jar);
        self.account_jars
            .entry(account_id.clone())
            .or_default()
            .jars
            .insert(jar);
    }

    fn insert_or_update_jar(&mut self, jar: Jar) {
        if jar.index < self.jars.len() {
            self.jars.replace(jar.index, jar);
        } else {
            self.jars.push(jar);
        }
    }
}
