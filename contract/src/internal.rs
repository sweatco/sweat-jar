use near_sdk::require;

use crate::*;

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

    pub(crate) fn get_product(&self, product_id: &ProductId) -> Product {
        self.products
            .get(product_id)
            .unwrap_or_else(|| env::panic_str(&format!("Product {product_id} doesn't exist")))
            .clone()
    }

    pub(crate) fn account_jar_ids(&self, account_id: &AccountId) -> Vec<JarIndex> {
        self.account_jars
            .get(account_id)
            .map_or_else(Vec::new, |items| items.iter().cloned().collect())
    }

    pub(crate) fn save_jar(&mut self, account_id: &AccountId, jar: &Jar) {
        self.insert_or_update_jar(jar);

        let mut indices = self
            .account_jars
            .get(account_id)
            .map_or_else(HashSet::new, |value| value.clone());
        indices.insert(jar.index);

        self.save_account_jars(account_id, indices);
    }

    pub(crate) fn save_account_jars(&mut self, account_id: &AccountId, indices: HashSet<JarIndex>) {
        if indices.is_empty() {
            self.account_jars.remove(account_id);
        } else {
            self.account_jars.insert(account_id.clone(), indices);
        }
    }

    fn insert_or_update_jar(&mut self, jar: &Jar) {
        if jar.index < self.jars.len() {
            self.jars.replace(jar.index, jar.clone());
        } else {
            self.jars.push(jar.clone());
        }
    }
}
