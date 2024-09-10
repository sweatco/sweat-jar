#![allow(deprecated)]

use near_sdk::{near_bindgen, AccountId};
use sweat_jar_model::api::MigrationToClaimRemainder;

use crate::{Contract, ContractExt};

#[near_bindgen]
impl MigrationToClaimRemainder for Contract {
    #[mutants::skip]
    fn migrate_accounts_to_claim_remainder(&mut self, accounts: Vec<AccountId>) {
        for account in accounts {
            self.migrate_account_jars_if_needed(&account);
        }
    }
}

impl Contract {
    /// Dynamic jars migration method
    #[mutants::skip]
    pub fn migrate_account_jars_if_needed(&mut self, account_id: &AccountId) {
        if let Some(jars) = self.account_jars_v1.remove(account_id) {
            self.account_jars.insert(account_id.clone(), jars.into());
        } else if let Some(jars) = self.account_jars_non_versioned.remove(account_id) {
            self.account_jars.insert(account_id.clone(), jars.into());
        };
    }
}

#[cfg(test)]
mod test {
    use near_sdk::test_utils::test_env::alice;

    use crate::{
        common::tests::Context,
        jar::{
            account_jars::{versioned::AccountJars, AccountJarsLastVersion},
            model::{AccountJarsLegacy, Jar, JarCache, JarLastVersion, JarLegacy},
        },
        migration::account_jars_non_versioned::AccountJarsNonVersioned,
        test_utils::admin,
    };

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

        contract.migrate_account_jars_if_needed(&alice());

        assert_eq!(
            contract.account_jars.get(&alice()).unwrap(),
            &AccountJars::V1(AccountJarsLastVersion {
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

        contract.migrate_account_jars_if_needed(&alice());

        assert_eq!(
            contract.account_jars.get(&alice()).unwrap(),
            &AccountJars::V1(AccountJarsLastVersion {
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
