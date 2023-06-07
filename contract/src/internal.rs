use crate::*;

impl Contract {
    pub(crate) fn assert_admin(&self) {
        assert!(
            self.admin_allowlist
                .contains(&env::predecessor_account_id()),
            "Can be performed only by admin"
        );
    }

    pub(crate) fn assert_product_exists(&self, product_id: &ProductId) {
        assert!(
            self.products.get(product_id).is_some(),
            "Product doesn't exist"
        );
    }

    pub(crate) fn get_jar(&self, index: JarIndex) -> Jar {
        self.jars
            .get(index)
            .expect(format!("Jar on index {} doesn't exist", index).as_str())
    }

    pub(crate) fn get_product(&self, product_id: &ProductId) -> Product {
        self.products
            .get(product_id)
            .expect(format!("Product {} doesn't exist", product_id).as_str())
    }

    pub(crate) fn account_jar_ids(&self, account_id: &AccountId) -> HashSet<JarIndex> {
        self.account_jars
            .get(account_id)
            .expect(format!("Account {} doesn't have jars", account_id).as_str())
    }

    pub(crate) fn save_jar(&mut self, account_id: &AccountId, jar: &Jar) {
        self.jars.push(jar);

        let mut indices = self.account_jars.get(&account_id).unwrap_or_default();
        indices.insert(jar.index);

        self.save_account_jars(&account_id, indices);
    }

    pub(crate) fn save_account_jars(&mut self, account_id: &AccountId, indices: HashSet<JarIndex>) {
        if indices.is_empty() {
            self.account_jars.remove(account_id);
        } else {
            self.account_jars.insert(account_id, &indices);
        }
    }
}
