use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{
    borsh::to_vec,
    env::{self, log_str, panic_str},
    json_types::Base64VecU8,
    near, require,
    serde_json::{self, json},
    AccountId, Gas, PromiseOrValue,
};
#[cfg(not(test))]
use near_sdk::{NearToken, Promise};
use sweat_jar_model::{
    account::{v1::AccountScore, versioned::AccountVersioned, Account},
    api::MigrationToV2,
    ScoreRecord, TokenAmount,
};

#[cfg(not(test))]
use crate::ft_interface::FungibleTokenInterface;
use crate::{
    assert::assert_not_locked,
    event::{emit, EventKind},
    internal::{assert_gas, is_promise_success},
    Contract, ContractExt, Roles,
};

const TGAS_FOR_MIGRATION_TRANSFER: u64 = 100;
const TGAS_FOR_MIGRATION_CALLBACK: u64 = 10;

#[near]
#[mutants::skip]
impl MigrationToV2 for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn force_migrate_account(&mut self, account_id: AccountId) -> PromiseOrValue<(AccountId, bool)> {
        self.migrate_account_inner(account_id)
    }

    fn migrate_account(&mut self) -> PromiseOrValue<(AccountId, bool)> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_inner(account_id)
    }

    fn is_account_locked(&self, account_id: AccountId) -> bool {
        self.migration.migrating_accounts.contains(&account_id)
    }

    #[access_control_any(roles(Roles::Maintainer))]
    fn unlock_account(&mut self, account_id: AccountId) {
        self.migration.migrating_accounts.remove(&account_id);
    }

    #[access_control_any(roles(Roles::Maintainer))]
    fn migrate_products(&mut self) -> PromiseOrValue<()> {
        let products: Vec<product_v2::Product> = self.products.values().map(Into::into).collect();
        let args = json!({
            "products": products
        });
        log_str(&format!("args: {args}"));
        let args_json = serde_json::to_vec(&args).unwrap_or_else(|_| panic_str("Failed to serialize args"));

        self.transfer_products(args_json)
    }
}

#[cfg(not(test))]
#[mutants::skip]
impl Contract {
    fn transfer_account(
        &mut self,
        account_id: &AccountId,
        principal: TokenAmount,
        memo: String,
        msg: String,
    ) -> PromiseOrValue<(AccountId, bool)> {
        self.ft_contract()
            .ft_transfer_call(
                &self.migration.new_version_account_id,
                principal,
                memo.as_str(),
                msg.as_str(),
                TGAS_FOR_MIGRATION_TRANSFER,
            )
            .then(Self::ext(env::current_account_id()).after_account_transferred(account_id.clone()))
            .into()
    }

    fn transfer_products(&mut self, args: Vec<u8>) -> PromiseOrValue<()> {
        Promise::new(self.migration.new_version_account_id.clone())
            .function_call(
                "migrate_products".to_string(),
                args,
                NearToken::from_yoctonear(0),
                Gas::from_tgas(TGAS_FOR_MIGRATION_TRANSFER),
            )
            .then(Self::ext(env::current_account_id()).after_products_migrated())
            .into()
    }
}

#[cfg(test)]
#[mutants::skip]
impl Contract {
    fn transfer_account(
        &mut self,
        account_id: &AccountId,
        _principal: TokenAmount,
        _memo: String,
        _msg: String,
    ) -> PromiseOrValue<(AccountId, bool)> {
        self.after_account_transferred(account_id.clone())
    }

    fn transfer_products(&mut self, _args: Vec<u8>) -> PromiseOrValue<()> {
        PromiseOrValue::Value(())
    }
}

#[near]
impl Contract {
    #[private]
    pub fn after_account_transferred(&mut self, account_id: AccountId) -> PromiseOrValue<(AccountId, bool)> {
        self.finalize_migration(account_id, is_promise_success())
    }

    #[private]
    pub fn after_products_migrated(&mut self) -> PromiseOrValue<()> {
        require!(is_promise_success(), "Products migration failed");

        PromiseOrValue::Value(())
    }
}
#[mutants::skip]
impl Contract {
    fn migrate_account_inner(&mut self, account_id: AccountId) -> PromiseOrValue<(AccountId, bool)> {
        self.assert_account_exists(&account_id);
        self.assert_account_is_not_migrating(&account_id);

        let Some((principal, memo, msg)) = self.prepare_migration_params(account_id.clone()) else {
            return self.finalize_migration(account_id, true);
        };

        assert_gas(
            Gas::from_tgas(TGAS_FOR_MIGRATION_TRANSFER + TGAS_FOR_MIGRATION_CALLBACK).as_gas(),
            || format!("Out of gas in migrate_account({account_id})"),
        );

        self.transfer_account(&account_id, principal, memo, msg)
    }

    fn prepare_migration_params(&mut self, account_id: AccountId) -> Option<(TokenAmount, String, String)> {
        let (account, principal) = self.map_legacy_account(account_id.clone());
        if account.jars.is_empty() {
            return None;
        }
        self.lock_account(&account_id);

        let account = AccountVersioned::V1(account);
        let account_vec: Base64VecU8 = to_vec(&account)
            .unwrap_or_else(|_| panic_str("Failed to serialize account"))
            .into();
        let memo = format!("migrate {account_id}");
        let msg = json!({
                "type": "migrate",
                "data": [
                    account_id.clone(),
                    account_vec,
                ]
        })
        .to_string();

        Some((principal, memo, msg))
    }

    fn finalize_migration(&mut self, account_id: AccountId, is_success: bool) -> PromiseOrValue<(AccountId, bool)> {
        if is_success {
            self.clear_account(&account_id);
            emit(EventKind::JarsMerge(account_id.clone()));
        }

        self.unlock_account(&account_id);

        PromiseOrValue::Value((self.migration.new_version_account_id.clone(), is_success))
    }

    fn map_legacy_account(&self, account_id: AccountId) -> (Account, TokenAmount) {
        let now = env::block_timestamp_ms();

        let score = self
            .get_score(&account_id)
            .map_or_else(ScoreRecord::default, crate::score::AccountScore::claimable_score);

        let mut account = Account {
            nonce: 0,
            score: self
                .get_score(&account_id)
                .map_or_else(AccountScore::default, |value| AccountScore {
                    updated: value.updated,
                    timezone: value.timezone,
                    scores: value.scores,
                    scores_history: value.scores_history,
                }),
            ..Account::default()
        };
        let mut total_principal = 0;

        let jars = self.account_jars(&account_id);
        for jar in &jars {
            assert_not_locked(jar);

            let updated_jar = account.deposit(&jar.product_id, jar.principal, jar.created_at.into());
            let (interest, remainder) = jar.get_interest(&score, &self.get_product(&jar.product_id), now);
            updated_jar.add_to_cache(now, interest, remainder);

            if !account.is_penalty_applied {
                account.is_penalty_applied = jar.is_penalty_applied;
            }

            total_principal += jar.principal;
        }

        (account, total_principal)
    }

    fn lock_account(&mut self, account_id: &AccountId) {
        self.migration.migrating_accounts.insert(account_id.clone());
    }

    fn unlock_account(&mut self, account_id: &AccountId) {
        self.migration.migrating_accounts.remove(account_id);
    }

    fn clear_account(&mut self, account_id: &AccountId) {
        self.accounts.remove(account_id);
        self.account_jars_v1.remove(account_id);
        self.account_jars_non_versioned.remove(account_id);
    }
}

#[cfg(test)]
#[mutants::skip]
mod tests {
    use near_sdk::test_utils::test_env::alice;
    use sweat_jar_model::ProductId;

    use super::*;
    use crate::{common::tests::Context, jar::model::Jar, product::model::Product, test_utils::admin};

    #[test]
    #[should_panic(expected = "Insufficient permissions for method force_migrate_account")]
    fn force_migrate_by_unauthorized_account() {
        let admin = admin();
        let alice = alice();

        let product = Product {
            id: "product".to_string(),
            ..Product::new()
        };

        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        context
            .contract()
            .create_jars(alice.clone(), "product".to_string(), 3 * 10u128.pow(18), 450);

        context.switch_account(alice.clone());
        let _ = context.contract().force_migrate_account(alice);
    }

    #[test]
    #[ignore]
    fn demo_prepare_migration_params() {
        let admin = admin();
        let alice = alice();

        let product_1 = Product {
            id: "product_1".to_string(),
            ..Product::new()
        };
        let product_2 = Product {
            id: "product_2".to_string(),
            ..Product::new()
        };
        let product_3 = Product {
            id: "product_3".to_string(),
            ..Product::new()
        };
        let product_4 = Product {
            id: "product_4".to_string(),
            ..Product::new()
        };

        let mut context = Context::new(admin.clone()).with_products(&[
            product_1.clone(),
            product_2.clone(),
            product_3.clone(),
            product_4.clone(),
        ]);

        context
            .contract()
            .create_jars(alice.clone(), "product_1".to_string(), 3 * 10u128.pow(18), 450);
        context
            .contract()
            .create_jars(alice.clone(), "product_2".to_string(), 5 * 10u128.pow(18), 50);
        context
            .contract()
            .create_jars(alice.clone(), "product_3".to_string(), 2 * 10u128.pow(18), 10);
        context
            .contract()
            .create_jars(alice.clone(), "product_4".to_string(), 1 * 10u128.pow(18), 10);

        context.switch_account(alice.clone());
        let (principal, memo, msg) = context.contract().prepare_migration_params(alice.clone()).unwrap();
        println!("principal: {principal}");
        println!("memo: {memo}");
        println!("msg: {msg}");
    }

    impl Contract {
        fn create_jars(
            &mut self,
            account_id: AccountId,
            product_id: ProductId,
            principal: TokenAmount,
            number_of_jars: u16,
        ) {
            let now = env::block_timestamp_ms();

            for _ in 0..number_of_jars {
                let id = self.increment_and_get_last_jar_id();
                let jar = Jar::create(id, account_id.clone(), product_id.clone(), principal, now);

                self.add_new_jar(&account_id, jar);
            }
        }
    }
}

#[mutants::skip]
mod product_v2 {
    use near_sdk::{
        json_types::{Base64VecU8, U128, U64},
        near,
    };
    use sweat_jar_model::{ProductId, Score, UDecimal as UDecimalLegacy};

    use crate::product::model::{
        Apy as ApyLegacy, Product as ProductLegacy, Terms as TermsLegacy, WithdrawalFee as WithdrawalFeeLegacy,
    };

    #[near(serializers=[json])]
    #[derive(Clone, Debug)]
    pub(super) struct Product {
        id: ProductId,
        cap: Cap,
        terms: Terms,
        withdrawal_fee: Option<WithdrawalFee>,
        public_key: Option<Base64VecU8>,
        is_enabled: bool,
    }

    #[near(serializers=[json])]
    #[derive(Clone, Debug)]
    struct Cap(U128, U128);

    #[near(serializers=[json])]
    #[derive(Clone, Debug, PartialEq)]
    #[serde(tag = "type", content = "data", rename_all = "snake_case")]
    enum WithdrawalFee {
        /// Describes a fixed amount of tokens that a user must pay as a fee on withdrawal.
        Fix(U128),

        /// Describes a percentage of the withdrawal amount that a user must pay as a fee on withdrawal.
        Percent(UDecimal),
    }

    #[near(serializers=[json])]
    #[derive(Clone, Debug, PartialEq)]
    #[serde(tag = "type", content = "data", rename_all = "snake_case")]
    enum Terms {
        Fixed(FixedProductTerms),
        Flexible(FlexibleProductTerms),
        ScoreBased(ScoreBasedProductTerms),
    }

    #[near(serializers=[json])]
    #[derive(Clone, Debug, PartialEq)]
    struct FixedProductTerms {
        lockup_term: U64,
        apy: Apy,
    }

    #[near(serializers=[json])]
    #[derive(Clone, Debug, PartialEq)]
    struct FlexibleProductTerms {
        apy: Apy,
    }

    #[near(serializers=[json])]
    #[derive(Clone, Debug, PartialEq)]
    struct ScoreBasedProductTerms {
        score_cap: Score,
        lockup_term: U64,
    }

    #[near(serializers=[json])]
    #[derive(Copy, Clone, Default, Debug, PartialEq)]
    struct UDecimal(U128, u32);

    /// A constant APY has no `fallback`; a downgradable one carries both rates.
    #[near(serializers=[json])]
    #[derive(Clone, Debug, PartialEq)]
    struct Apy {
        default: UDecimal,
        #[serde(skip_serializing_if = "Option::is_none")]
        fallback: Option<UDecimal>,
    }

    impl From<ApyLegacy> for Apy {
        fn from(value: ApyLegacy) -> Self {
            match value {
                ApyLegacy::Constant(value) => Self {
                    default: value.into(),
                    fallback: None,
                },
                ApyLegacy::Downgradable(value) => Self {
                    default: value.default.into(),
                    fallback: Some(value.fallback.into()),
                },
            }
        }
    }

    impl From<UDecimalLegacy> for UDecimal {
        fn from(value: UDecimalLegacy) -> Self {
            UDecimal(value.significand.into(), value.exponent)
        }
    }

    impl From<ProductLegacy> for Product {
        fn from(value: ProductLegacy) -> Self {
            let terms: Terms = match value.terms {
                TermsLegacy::Fixed(terms) => {
                    if value.score_cap > 0 {
                        Terms::ScoreBased(ScoreBasedProductTerms {
                            lockup_term: terms.lockup_term.into(),

                            score_cap: value.score_cap,
                        })
                    } else {
                        Terms::Fixed(FixedProductTerms {
                            lockup_term: terms.lockup_term.into(),
                            apy: value.apy.into(),
                        })
                    }
                }

                TermsLegacy::Flexible => Terms::Flexible(FlexibleProductTerms { apy: value.apy.into() }),
            };

            Self {
                id: value.id,
                cap: Cap(value.cap.min.into(), value.cap.max.into()),
                terms,
                withdrawal_fee: value.withdrawal_fee.map(|fee| match fee {
                    WithdrawalFeeLegacy::Fix(amount) => WithdrawalFee::Fix(amount.into()),
                    WithdrawalFeeLegacy::Percent(percentage) => WithdrawalFee::Percent(percentage.into()),
                }),
                public_key: value.public_key.map(Into::into),
                is_enabled: value.is_enabled,
            }
        }
    }
}
