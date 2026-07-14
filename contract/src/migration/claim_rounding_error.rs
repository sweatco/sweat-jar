#![allow(deprecated)]

use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{near, AccountId};
use sweat_jar_model::api::MigrationToClaimRemainder;

use crate::{Contract, ContractExt, Roles};

#[near]
impl MigrationToClaimRemainder for Contract {
    #[mutants::skip]
    #[access_control_any(roles(Roles::Maintainer))]
    fn migrate_accounts_to_claim_remainder(&mut self, accounts: Vec<AccountId>) {
        for account in accounts {
            self.migrate_account_if_needed(&account);
        }
    }
}

impl Contract {
    /// Dynamic jars migration method
    #[mutants::skip]
    pub fn migrate_account_if_needed(&mut self, account_id: &AccountId) {
        if let Some(jars) = self.account_jars_v1.remove(account_id) {
            self.accounts.insert(account_id.clone(), jars.into());
        } else if let Some(jars) = self.account_jars_non_versioned.remove(account_id) {
            self.accounts.insert(account_id.clone(), jars.into());
        }
    }
}

#[cfg(test)]
mod test {
    use near_sdk::test_utils::test_env::alice;
    use sweat_jar_model::api::MigrationToClaimRemainder;

    use crate::{
        common::tests::Context,
        jar::{
            account::{versioned::Account, AccountJarsLastVersion},
            model::{AccountJarsLegacy, Jar, JarCache, JarLastVersion, JarLegacy},
        },
        migration::account_jars_non_versioned::AccountJarsNonVersioned,
        test_utils::admin,
    };

    #[test]
    #[should_panic(expected = "Insufficient permissions for method migrate_accounts_to_claim_remainder")]
    fn migrate_accounts_to_claim_remainder_by_non_maintainer() {
        let context = Context::new(admin());
        // The default caller ("owner") holds no roles.
        context.contract().migrate_accounts_to_claim_remainder(vec![alice()]);
    }

    #[test]
    fn migrate_accounts_to_claim_remainder_by_maintainer() {
        let admin = admin();
        let mut context = Context::new(admin.clone());
        context.switch_account(&admin);

        context.contract().migrate_accounts_to_claim_remainder(vec![alice()]);
    }

    #[test]
    fn account_jars_legacy_migration() {
        let ctx = Context::new(admin());
        let mut contract = ctx.contract();

        contract.account_jars_v1.insert(
            alice(),
            AccountJarsLegacy {
                last_id: 5,
                jars: vec![JarLegacy {
                    id: 5,
                    account_id: alice(),
                    product_id: "product".to_string(),
                    created_at: 5,
                    principal: 6,
                    cache: Some(JarCache {
                        updated_at: 55,
                        interest: 99,
                    }),
                    claimed_balance: 7,
                    is_pending_withdraw: true,
                    is_penalty_applied: true,
                }],
            },
        );

        contract.migrate_account_if_needed(&alice());

        assert_eq!(
            contract.accounts.get(&alice()).unwrap(),
            &Account::V1(AccountJarsLastVersion {
                last_id: 5,
                jars: vec![Jar::V1(JarLastVersion {
                    id: 5,
                    account_id: alice(),
                    product_id: "product".to_string(),
                    created_at: 5,
                    principal: 6,
                    cache: Some(JarCache {
                        updated_at: 55,
                        interest: 99,
                    }),
                    claimed_balance: 7,
                    is_pending_withdraw: true,
                    is_penalty_applied: true,
                    claim_remainder: 0,
                })],
                score: Default::default(),
            })
        )
    }

    #[test]
    fn account_jars_non_versioned_migration() {
        let ctx = Context::new(admin());
        let mut contract = ctx.contract();

        contract.account_jars_non_versioned.insert(
            alice(),
            AccountJarsNonVersioned {
                last_id: 5,
                jars: vec![Jar::V1(JarLastVersion {
                    id: 5,
                    account_id: alice(),
                    product_id: "product".to_string(),
                    created_at: 5,
                    principal: 6,
                    cache: Some(JarCache {
                        updated_at: 55,
                        interest: 99,
                    }),
                    claimed_balance: 7,
                    is_pending_withdraw: true,
                    is_penalty_applied: true,
                    claim_remainder: 0,
                })],
            },
        );

        contract.migrate_account_if_needed(&alice());

        assert_eq!(
            contract.accounts.get(&alice()).unwrap(),
            &Account::V1(AccountJarsLastVersion {
                last_id: 5,
                jars: vec![Jar::V1(JarLastVersion {
                    id: 5,
                    account_id: alice(),
                    product_id: "product".to_string(),
                    created_at: 5,
                    principal: 6,
                    cache: Some(JarCache {
                        updated_at: 55,
                        interest: 99,
                    }),
                    claimed_balance: 7,
                    is_pending_withdraw: true,
                    is_penalty_applied: true,
                    claim_remainder: 0,
                })],
                score: Default::default(),
            })
        )
    }
}
