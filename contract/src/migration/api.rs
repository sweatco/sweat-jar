use std::ops::Deref;

use near_sdk::{env, env::panic_str, json_types::U128, require, AccountId};
use sweat_jar_model::{api::JarApi, jar::CeFiJar, ProductId, TokenAmount};

use crate::{
    event::{emit, EventKind, MigrationEventItem},
    jar::{
        account::v2::AccountV2,
        model::{JarCache, JarLastVersion},
    },
    product::model::v2::InterestCalculator,
    Contract,
};

impl Contract {
    /// Migrates `CeFi Jars` to create `DeFi Jars`.
    ///
    /// This method receives a list of entities called `CeFiJar`, which represent token deposits
    /// from a 3rd party service, and creates corresponding `DeFi Jars` for them. In order to support
    /// the transition of deposit terms from the 3rd party to the contract, the `Product` with these
    /// terms must be registered beforehand.
    ///
    /// # Arguments
    ///
    /// - `jars`: A vector of `CeFiJar` entities representing token deposits from a 3rd party service.
    /// - `total_received`: The total amount of tokens received, ensuring that all tokens are distributed
    ///   correctly.
    ///
    /// # Panics
    ///
    /// This method can panic in following cases:
    ///
    /// 1. If a `Product` required to create a Jar is not registered. In such a case, the migration
    ///    process cannot proceed, and the method will panic.
    ///
    /// 2. If the total amount of tokens received is not equal to the sum of all `CeFiJar` entities.
    ///    This panic ensures that all deposits are properly migrated, and any discrepancies will trigger
    ///    an error.
    ///
    /// 3. Panics in case of unauthorized access by non-admin users.
    ///
    /// # Authorization
    ///
    /// This method can only be called by the Contract Admin. Unauthorized access will result in a panic.
    ///
    pub(crate) fn migrate_jars(&mut self, jars: Vec<CeFiJar>, total_received: U128) {
        let mut event_data: Vec<MigrationEventItem> = vec![];
        let mut total_amount: TokenAmount = 0;

        for ce_fi_jar in jars {
            require!(
                self.products.get(&ce_fi_jar.product_id).is_some(),
                format!("Product {} is not registered", ce_fi_jar.product_id),
            );

            let id = self.increment_and_get_last_jar_id();

            self.assert_migrated(&ce_fi_jar.account_id);
            let account_jars = self.accounts.entry(ce_fi_jar.account_id.clone()).or_default();

            let jar = JarLastVersion {
                id,
                account_id: ce_fi_jar.account_id,
                product_id: ce_fi_jar.product_id,
                created_at: ce_fi_jar.created_at.0,
                principal: ce_fi_jar.principal.0,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            };

            total_amount += jar.principal;

            event_data.push(MigrationEventItem {
                original_id: ce_fi_jar.id,
                id: jar.id,
                account_id: jar.account_id.clone(),
            });

            account_jars.push(jar.into());
        }

        require!(
            total_received.0 == total_amount,
            "Total received doesn't match the sum of principals"
        );

        emit(EventKind::Migration(event_data));
    }

    pub fn migrate_account(&mut self) {
        let account_id = env::predecessor_account_id();

        let Some(account) = self.get_account_legacy(&account_id) else {
            panic_str("No legacy account");
        };

        require!(!self.accounts_v2.contains_key(&account_id), "Account already exists");

        let now = env::block_timestamp_ms();
        let mut account = AccountV2::from(account.deref());

        let interest: Vec<(ProductId, TokenAmount, u64)> = account
            .jars
            .iter()
            .map(|(product_id, jar)| {
                let product = self.get_product(product_id);
                let (interest, remainder) = product.terms.get_interest(&account, jar, now);

                (product_id.clone(), interest - jar.claimed_balance, remainder)
            })
            .collect();

        for (product_id, interest, remainder) in interest {
            let jar = account.get_jar_mut(&product_id);
            jar.cache = Some(JarCache {
                updated_at: now,
                interest,
            });
            jar.claim_remainder = remainder;
        }

        self.accounts_v2.insert(account_id.clone(), account);
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::test_env::alice;
    use sweat_jar_model::{UDecimal, MS_IN_YEAR};

    use crate::{
        common::tests::Context,
        jar::{
            account::{v1::AccountV1, versioned::Account},
            model::{Jar, JarCache, JarLastVersion, JarVersioned},
        },
        product::model::{
            v2::{Apy, FixedProductTerms, Terms},
            ProductV2,
        },
        test_utils::admin,
    };

    #[test]
    fn migrate_legacy_account() {
        let product = ProductV2::new().with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR,
            apy: Apy::Constant(UDecimal::new(10_000, 5)),
        }));
        let mut context = Context::new(admin()).with_products(&[product.clone()]);

        /* Jar 1:
         * - create at 0 with 500_000
         * - claim at YEAR / 4 --> 12_500
         * --> target interest at (YEAR / 2) is (25_000 - 12_500 = 12_500)
         * Jar 2:
         * - create at YEAR / 5 with 10_000
         * --> target interest at (YEAR / 2) is 300
         */
        let jars: Vec<Jar> = vec![
            JarVersioned::V1(JarLastVersion {
                id: 0,
                account_id: alice(),
                product_id: product.id.clone(),
                created_at: 0,
                principal: 500_000,
                cache: Some(JarCache {
                    updated_at: MS_IN_YEAR / 4,
                    interest: 0,
                }),
                claimed_balance: 12_500,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            }),
            JarVersioned::V1(JarLastVersion {
                id: 1,
                account_id: alice(),
                product_id: product.id.clone(),
                created_at: MS_IN_YEAR / 5,
                principal: 10_000,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            }),
        ];
        let account = Account::V1(AccountV1 {
            last_id: 1,
            score: Default::default(),
            jars,
        });
        context.contract().accounts.insert(alice(), account);

        let migration_time = MS_IN_YEAR / 2;
        context.set_block_timestamp_in_ms(migration_time);
        context.switch_account(&alice());
        context.contract().migrate_account();

        let contract = context.contract();
        let account = contract.get_account(&alice());
        assert_eq!(1, account.jars.len());

        let jar = account.get_jar(&product.id);
        assert_eq!(2, jar.deposits.len());
        assert_eq!(migration_time, jar.cache.unwrap().updated_at);
        assert_eq!(12_800, jar.cache.unwrap().interest);
    }
}
