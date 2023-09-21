use near_sdk::require;

use crate::{env, jar::model::JarID, AccountId, Contract, Jar, Product, ProductId};

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

    pub(crate) fn next_jar_id(&mut self) -> JarID {
        self.last_jar_id += 1;
        self.last_jar_id
    }

    pub(crate) fn increment_jar_id(&mut self) {
        self.last_jar_id += 1;
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

    pub(crate) fn account_jars(&self, account_id: &AccountId) -> &[Jar] {
        self.account_jars.get(account_id).unwrap_or(&self.empty_jars)
    }

    pub(crate) fn save_jar(&mut self, account_id: &AccountId, jar: Jar) {
        self.account_jars.entry(account_id.clone()).or_default().push(jar);
    }
}
