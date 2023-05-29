use crate::*;

impl Contract {
    pub(crate) fn assert_admin(&self) {
        assert!(self.admin_allowlist.contains(&env::predecessor_account_id()), "Can be performed only by admin");
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
