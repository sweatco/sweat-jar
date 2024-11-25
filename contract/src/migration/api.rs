use near_sdk::{env, env::panic_str, near_bindgen, require};
use sweat_jar_model::{ProductId, TokenAmount};

use crate::{
    jar::{
        account::{versioned::AccountVersioned, Account},
        model::JarCache,
    },
    product::model::InterestCalculator,
    Contract, ContractExt,
};

#[near_bindgen]
impl Contract {
    pub fn migrate_account(&mut self) {
        let account_id = env::predecessor_account_id();

        let Some(account) = self.archive.get_account(&account_id) else {
            panic_str("No legacy account");
        };

        require!(!self.accounts.contains_key(&account_id), "Account already exists");

        let now = env::block_timestamp_ms();
        let mut account = Account::from(&account);

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

        self.accounts.insert(account_id.clone(), AccountVersioned::new(account));
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::test_env::alice;
    use sweat_jar_model::{UDecimal, MS_IN_YEAR};

    use crate::{
        common::tests::Context,
        jar::model::{AccountLegacyV2, Jar, JarCache, JarLastVersion},
        product::model::{Apy, FixedProductTerms, Product, Terms},
        test_utils::admin,
    };

    #[test]
    fn migrate_legacy_account() {
        let product = Product::new().with_terms(Terms::Fixed(FixedProductTerms {
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
            Jar::V1(JarLastVersion {
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
            Jar::V1(JarLastVersion {
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

        let account = AccountLegacyV2 { last_id: 1, jars };
        context.contract().archive.accounts_v2.insert(alice(), account);

        let migration_time = MS_IN_YEAR / 2;
        context.set_block_timestamp_in_ms(migration_time);
        context.switch_account(alice());
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
