use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{
    borsh::to_vec,
    env::{self, log_str, panic_str},
    json_types::{Base64VecU8, U128},
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
            .then(Self::ext(env::current_account_id()).after_account_transferred(account_id.clone(), U128(principal)))
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

    fn migration_fully_transferred(principal: TokenAmount) -> bool {
        env::promise_result_checked(0, 128)
            .ok()
            .and_then(|value| serde_json::from_slice::<U128>(&value).ok())
            .is_some_and(|used| used.0 == principal)
    }
}

#[cfg(test)]
#[mutants::skip]
impl Contract {
    fn transfer_account(
        &mut self,
        account_id: &AccountId,
        principal: TokenAmount,
        _memo: String,
        _msg: String,
    ) -> PromiseOrValue<(AccountId, bool)> {
        self.after_account_transferred(account_id.clone(), U128(principal))
    }

    fn transfer_products(&mut self, _args: Vec<u8>) -> PromiseOrValue<()> {
        PromiseOrValue::Value(())
    }

    fn migration_fully_transferred(principal: TokenAmount) -> bool {
        if !is_promise_success() {
            return false;
        }

        crate::common::test_data::get_test_migration_used_amount().unwrap_or(principal) == principal
    }
}

#[near]
impl Contract {
    #[private]
    pub fn after_account_transferred(
        &mut self,
        account_id: AccountId,
        principal: U128,
    ) -> PromiseOrValue<(AccountId, bool)> {
        let fully_transferred = Self::migration_fully_transferred(principal.0);
        self.finalize_migration(account_id, fully_transferred)
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

        // The interest cached below already includes this score. Consume it on the
        // account copy sent to v2 so the destination cannot use the same score to
        // calculate interest a second time. The source state stays untouched until
        // the cross-contract transfer has fully succeeded.
        let mut migrated_score = self.get_score(&account_id).copied().unwrap_or_default();
        if migrated_score.is_valid() {
            let score = migrated_score.claimable_score();
            migrated_score.claim_score();
            self.map_legacy_account_with_score(account_id, now, migrated_score, score)
        } else {
            // Accounts without score jars have the default, invalid timezone. They
            // have no usable score to preserve, but the payload must still not carry
            // a claimable buffer.
            migrated_score.scores = [0; 2];
            self.map_legacy_account_with_score(account_id, now, migrated_score, ScoreRecord::default())
        }
    }

    fn map_legacy_account_with_score(
        &self,
        account_id: AccountId,
        now: u64,
        migrated_score: crate::score::AccountScore,
        score: ScoreRecord,
    ) -> (Account, TokenAmount) {
        let mut account = Account {
            nonce: 0,
            score: AccountScore {
                updated: migrated_score.updated,
                timezone: migrated_score.timezone,
                scores: migrated_score.scores,
                scores_history: migrated_score.scores_history,
            },
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
    use sweat_jar_model::{ProductId, Timezone};

    use super::*;
    use crate::{common::tests::Context, jar::model::Jar, product::model::Product, test_utils::admin};

    #[test]
    fn is_account_locked_reflects_migration_state() {
        let admin = admin();
        let alice = alice();
        let context = Context::new(admin);

        assert!(!context.contract().is_account_locked(alice.clone()));

        context.contract().migration.migrating_accounts.insert(alice.clone());
        assert!(context.contract().is_account_locked(alice));
    }

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

    /// A context with `alice` holding three 1_000_000 jars (3_000_000 principal
    /// total) and `admin` (a Maintainer) set as the caller, ready to migrate.
    fn context_ready_to_migrate() -> (AccountId, Context) {
        let admin = admin();
        let alice = alice();
        let product = Product {
            id: "product".to_string(),
            ..Product::new()
        };

        let mut context = Context::new(admin.clone()).with_products(&[product]);
        context
            .contract()
            .create_jars(alice.clone(), "product".to_string(), 1_000_000, 3);
        context.switch_account(admin);

        (alice, context)
    }

    #[test]
    fn migrate_account_clears_it_when_full_principal_is_accepted() {
        let (alice, context) = context_ready_to_migrate();

        crate::common::test_data::set_test_future_success(true);
        let _ = context.contract().force_migrate_account(alice.clone());

        assert!(context.contract().accounts.get(&alice).is_none());
    }

    #[test]
    fn migrate_account_keeps_it_when_only_part_is_accepted() {
        let (alice, context) = context_ready_to_migrate();

        // The transfer promise resolves, but the v2 contract accepts less than the
        // full 3_000_000 principal — the account must not be cleared.
        crate::common::test_data::set_test_future_success(true);
        crate::common::test_data::set_test_migration_used_amount(2_999_999);
        let _ = context.contract().force_migrate_account(alice.clone());

        assert!(context.contract().accounts.get(&alice).is_some());
        assert!(!context.contract().migration.migrating_accounts.contains(&alice));
    }

    #[test]
    fn migrate_account_keeps_it_when_transfer_fails() {
        let (alice, context) = context_ready_to_migrate();

        crate::common::test_data::set_test_future_success(false);
        let _ = context.contract().force_migrate_account(alice.clone());

        assert!(context.contract().accounts.get(&alice).is_some());
        assert!(!context.contract().migration.migrating_accounts.contains(&alice));
    }

    #[test]
    fn migration_payload_consumes_score_without_mutating_source_account() {
        let (alice, context) = context_ready_to_migrate();

        let source_score = {
            let mut contract = context.contract();
            let score = &mut contract.accounts.entry(alice.clone()).or_default().score;
            score.timezone = Timezone::hour_shift(0);
            score.scores = [111, 222];
            score.scores_history = [333, 444];
            *score
        };
        let mut expected_payload_score = source_score;
        expected_payload_score.claim_score();

        let contract = context.contract();
        let (account, _) = contract.map_legacy_account(alice.clone());

        assert_eq!(account.score.updated, expected_payload_score.updated);
        assert_eq!(account.score.timezone, expected_payload_score.timezone);
        assert_eq!(account.score.scores, expected_payload_score.scores);
        assert_eq!(account.score.scores_history, expected_payload_score.scores_history);
        assert_eq!(*contract.get_score(&alice).unwrap(), source_score);
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
