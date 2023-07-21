use crate::*;

impl Contract {
    pub(crate) fn assert_admin(&self) {
        assert!(
            self.admin_allowlist
                .contains(&env::predecessor_account_id()),
            "Can be performed only by admin"
        );
    }

    pub(crate) fn get_product(&self, product_id: &ProductId) -> Product {
        self.products
            .get(product_id)
            .unwrap_or_else(|| panic!("Product {} doesn't exist", product_id))
    }

    pub(crate) fn account_jar_ids(&self, account_id: &AccountId) -> Vec<JarIndex> {
        self.account_jars
            .get(account_id)
            .unwrap_or_else(|| panic!("Account {} doesn't have jars", account_id))
            .iter()
            .cloned()
            .collect()
    }

    pub(crate) fn save_jar(&mut self, account_id: &AccountId, jar: &Jar) {
        self.insert_or_update_jar(jar);

        let mut indices = self.account_jars.get(&account_id).unwrap_or_default();
        indices.insert(jar.index);

        self.save_account_jars(account_id, indices);
    }

    pub(crate) fn save_account_jars(&mut self, account_id: &AccountId, indices: HashSet<JarIndex>) {
        if indices.is_empty() {
            self.account_jars.remove(account_id);
        } else {
            self.account_jars.insert(account_id, &indices);
        }
    }

    fn insert_or_update_jar(&mut self, jar: &Jar) {
        if jar.index < self.jars.len() {
            self.jars.replace(jar.index, jar);
        } else {
            self.jars.push(jar);
        }
    }
}
