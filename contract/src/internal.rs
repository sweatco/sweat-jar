use std::fmt::Display;

use near_sdk::require;
use sweat_jar_model::jar::JarId;

use crate::{
    env,
    jar::{account::versioned::Account, model::Jar},
    AccountId, Contract,
};

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

    pub(crate) fn assert_account_can_update(&self) {
        self.assert_manager();
    }

    pub(crate) fn increment_and_get_last_jar_id(&mut self) -> JarId {
        self.last_jar_id += 1;
        self.last_jar_id
    }

    pub(crate) fn account_jars(&self, account_id: &AccountId) -> Option<Vec<Jar>> {
        // TODO: Remove after complete migration and return '&[Jar]`
        if let Some(record) = self.account_jars_v1.get(account_id) {
            return Some(record.jars.iter().map(|j| j.clone().into()).collect());
        }

        if let Some(record) = self.account_jars_non_versioned.get(account_id) {
            return Some(record.jars.clone());
        }

        self.accounts.get(account_id).map(|record| record.jars.clone())
    }

    pub(crate) fn get_account_legacy(&self, account_id: &AccountId) -> Option<&Account> {
        if let Some(record) = self.account_jars_v1.get(account_id) {
            return Account::from(record).into();
        }

        if let Some(record) = self.account_jars_non_versioned.get(account_id) {
            return Account::from(record).into();
        }

        self.accounts.get(account_id)
    }

    pub(crate) fn add_new_jar(&mut self, account_id: &AccountId, jar: Jar) {
        self.migrate_account_if_needed(account_id);
        let jars = self.accounts.entry(account_id.clone()).or_default();
        jars.last_id = jar.id;
        jars.push(jar);
    }
}

pub(crate) fn assert_gas<Message: Display>(gas_needed: u64, error: impl FnOnce() -> Message) {
    let gas_left = env::prepaid_gas().as_gas() - env::used_gas().as_gas();

    if gas_left < gas_needed {
        let error = error();

        env::panic_str(&format!(
            r#"Not enough gas left. Consider attaching more gas to the transaction.
               {error}
               Gas left: {gas_left} Needed: {gas_needed}. Need additional {} gas"#,
            gas_needed - gas_left
        ));
    }
}

#[cfg(test)]
mod test {
    use near_sdk::env;

    use crate::{
        common::tests::Context,
        internal::assert_gas,
        test_utils::{admin, expect_panic},
    };

    #[test]
    #[should_panic(expected = r#"Can be performed only by admin"#)]
    fn self_update_without_access() {
        let admin = admin();
        let context = Context::new(admin);
        context.contract().update_contract(vec![], None);
    }

    #[test]
    fn test_assert_gas() {
        const GAS_FOR_ASSERT_CALL: u64 = 529536222;

        expect_panic(
            &(),
            "Not enough gas left. Consider attaching more gas to the transaction.",
            || {
                assert_gas(u64::MAX, || "Error message");
            },
        );

        let gas_left = env::prepaid_gas().as_gas() - env::used_gas().as_gas();
        expect_panic(&(), &format!("Need additional {GAS_FOR_ASSERT_CALL} gas"), || {
            assert_gas(gas_left, || "Error message");
        });

        let gas_left = env::prepaid_gas().as_gas() - env::used_gas().as_gas();
        expect_panic(&(), "Need additional 1 gas", || {
            assert_gas(gas_left - GAS_FOR_ASSERT_CALL + 1, || "Error message");
        });

        let gas_left = env::prepaid_gas().as_gas() - env::used_gas().as_gas();
        assert_gas(gas_left - GAS_FOR_ASSERT_CALL, || "Error message");

        let gas_left = env::prepaid_gas().as_gas() - env::used_gas().as_gas();
        assert_gas(gas_left - GAS_FOR_ASSERT_CALL - 1, || "Error message");
    }
}

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
pub fn is_promise_success() -> bool {
    near_sdk::is_promise_success()
}

#[cfg(test)]
pub fn is_promise_success() -> bool {
    crate::common::test_data::get_test_future_success()
}
