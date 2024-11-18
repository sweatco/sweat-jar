#![cfg(test)]

use fake::Fake;
use near_sdk::{
    store::LookupMap,
    test_utils::test_env::{alice, bob},
    AccountId, PromiseOrValue,
};
use sweat_jar_model::{
    api::{ClaimApi, JarApi, ScoreApi, WithdrawApi},
    withdraw::WithdrawView,
    ProductId, Score, Timezone, TokenAmount, UDecimal, MS_IN_DAY, UTC,
};

use crate::{
    common::{
        test_data::{set_test_future_success, set_test_log_events},
        tests::Context,
    },
    jar::model::JarV2,
    product::model::{Apy, Cap, FixedProductTerms, InterestCalculator, ProductV2, ScoreBasedProductTerms, Terms},
    score::AccountScore,
    test_utils::admin,
    StorageKey,
};

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn record_score_by_non_manager() {
    let mut context = Context::new(admin());

    context.switch_account(alice());
    context.contract().record_score(vec![(alice(), vec![(100, 0.into())])]);
}

/// 12% jar should have the same interest as 12_000 score jar walking to the limit every day
/// Also this method tests score cap
#[test]
fn same_interest_in_score_jar_as_in_const_jar() {
    set_test_log_events(false);

    let term_in_days: u64 = 365;
    let term_in_ms: u64 = term_in_days * MS_IN_DAY;
    let half_period: u64 = term_in_days / 2;

    let regular_product = ProductV2 {
        id: "regular_product".to_string(),
        cap: Cap { min: 0, max: 1_000_000 },
        terms: Terms::Fixed(FixedProductTerms {
            lockup_term: term_in_ms,
            apy: Apy::Constant(UDecimal::new(12000, 5)),
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    let score_product = ProductV2 {
        id: "score_product".to_string(),
        cap: Cap { min: 0, max: 1_000_000 },
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms,
            base_apy: Apy::Constant(UDecimal::zero()),
            score_cap: 12_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    let mut context = Context::new(admin())
        .with_products(&[regular_product.clone(), score_product.clone()])
        .with_jars(
            &alice(),
            &[
                (regular_product.id.clone(), JarV2::new().with_deposit(0, 100)),
                (score_product.id.clone(), JarV2::new().with_deposit(0, 100)),
            ],
        );
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(3));

    // Difference of 1 is okay because the missing yoctosweat is stored in claim remainder
    // and will eventually be added to total claimed balance
    fn compare_interest(context: &Context, regular_product_id: &ProductId, score_product_id: &ProductId) {
        let regular_interest = context.interest(&alice(), regular_product_id);
        let score_interest = context.interest(&alice(), score_product_id);
        let diff = regular_interest.abs_diff(score_interest);

        assert!(diff <= 1, "Diff is too big {diff}");
    }

    let mut total_claimed = 0;

    for day in 0..term_in_days {
        let now = day * MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);
        context.record_score(&alice(), UTC(day * MS_IN_DAY), 20_000);

        compare_interest(&context, &regular_product.id, &score_product.id);

        if day == half_period {
            let jar_interest = context.interest(&alice(), &regular_product.id);
            let score_interest = context.interest(&alice(), &score_product.id);

            let claimed = context.claim_total(&alice());

            total_claimed += claimed;

            assert_eq!(claimed, jar_interest + score_interest);
        }
    }

    assert_eq!(
        context.jar(&alice(), &regular_product.id).cache.unwrap().updated_at,
        half_period * MS_IN_DAY
    );
    assert_eq!(
        context.jar(&alice(), &score_product.id).cache.unwrap().updated_at,
        (term_in_days - 1) * MS_IN_DAY
    );

    context.set_block_timestamp_in_ms(term_in_ms);
    compare_interest(&context, &regular_product.id, &score_product.id);

    total_claimed += context.claim_total(&alice());
    assert_eq!(total_claimed, 24);
}

// TODO: it fails with bigger deposits
#[test]
fn score_jar_claim_often_vs_claim_at_the_end() {
    set_test_log_events(false);

    let term_in_days = 365;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = ProductV2 {
        id: "score_product".to_string(),
        cap: Cap {
            min: 0,
            max: 1_000_000_000_000_000,
        },
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms,
            base_apy: Apy::Constant(UDecimal::zero()),
            score_cap: 20_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), JarV2::new().with_deposit(0, 100))])
        .with_jars(&bob(), &[(product.id.clone(), JarV2::new().with_deposit(0, 100))]);
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));
    context.contract().get_account_mut(&bob()).score = AccountScore::new(Timezone::hour_shift(0));

    fn update_and_check(day: u64, context: &mut Context, total_claimed_bob: &mut u128, product_id: &ProductId) {
        let score: Score = (0..1000).fake();

        context.switch_account(admin());
        context.record_score(&alice(), UTC(day * MS_IN_DAY), score);
        context.record_score(&bob(), UTC(day * MS_IN_DAY), score);

        if day > 1 {
            context.switch_account(admin());
            context.record_score(&alice(), UTC((day - 1) * MS_IN_DAY), score);
            context.record_score(&bob(), UTC((day - 1) * MS_IN_DAY), score);
        }

        *total_claimed_bob += context.claim_total(&bob());
        assert_eq!(context.interest(&alice(), &product_id), *total_claimed_bob, "{day}");
    }

    let mut total_claimed_bob: u128 = 0;

    // Update each hour for 10 days
    for hour in 0..(24 * 10) {
        context.set_block_timestamp_in_hours(hour);
        update_and_check(hour / 24, &mut context, &mut total_claimed_bob, &product.id);
    }

    // Update each day until 100 days has passed
    for day in 10..100 {
        context.set_block_timestamp_in_days(day);
        update_and_check(day, &mut context, &mut total_claimed_bob, &product.id);
    }

    total_claimed_bob += context.claim_total(&bob());

    assert_eq!(context.interest(&alice(), &product.id), total_claimed_bob);
    assert_eq!(context.claim_total(&alice()), total_claimed_bob);

    assert_eq!(
        context.jar(&alice(), &product.id).cache.unwrap().updated_at,
        MS_IN_DAY * 99
    );
}

#[test]
fn interest_does_not_increase_with_no_steps() {
    set_test_log_events(false);

    let term_in_days = 365;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = ProductV2 {
        id: "score_product".to_string(),
        cap: Cap {
            min: 0,
            max: 1_000_000_000_000_000,
        },
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms,
            base_apy: Apy::Constant(UDecimal::zero()),
            score_cap: 20_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    let mut context = Context::new(admin()).with_products(&[product.clone()]).with_jars(
        &alice(),
        &[(product.id.clone(), JarV2::new().with_deposit(0, 100_000_000))],
    );
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));

    context.set_block_timestamp_in_days(5);

    context.record_score(&alice(), UTC(5 * MS_IN_DAY), 1000);

    assert_eq!(context.interest(&alice(), &product.id), 0);

    context.set_block_timestamp_in_days(6);

    let interest_for_one_day = context.interest(&alice(), &product.id);
    assert_ne!(interest_for_one_day, 0);

    context.set_block_timestamp_in_days(7);
    assert_eq!(interest_for_one_day, context.interest(&alice(), &product.id));

    context.set_block_timestamp_in_days(50);
    assert_eq!(interest_for_one_day, context.interest(&alice(), &product.id));

    context.set_block_timestamp_in_days(100);
    assert_eq!(interest_for_one_day, context.interest(&alice(), &product.id));
}

#[test]
fn withdraw_score_jar() {
    set_test_log_events(false);

    let term_in_days = 7;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = ProductV2 {
        id: "score_product".to_string(),
        cap: Cap {
            min: 0,
            max: 1_000_000_000_000_000,
        },
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms,
            base_apy: Apy::Constant(UDecimal::zero()),
            score_cap: 20_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), JarV2::new().with_deposit(0, 100))])
        .with_jars(&bob(), &[(product.id.clone(), JarV2::new().with_deposit(0, 100))]);
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));
    context.contract().get_account_mut(&bob()).score = AccountScore::new(Timezone::hour_shift(0));

    for i in 0..=10 {
        context.set_block_timestamp_in_days(i);

        context.record_score(&alice(), (i * MS_IN_DAY).into(), 1000);
        context.record_score(&bob(), (i * MS_IN_DAY).into(), 1000);

        if i == 5 {
            let claimed_alice = context.claim_total(&alice());
            let claimed_bob = context.claim_total(&bob());
            assert_eq!(claimed_alice, claimed_bob);
        }
    }

    // Alice claims first and then withdraws
    let claimed_alice = context.claim_total(&alice());
    let withdrawn_alice = context.withdraw(&alice(), &product.id);

    assert_eq!(context.claim_total(&alice()), 0);

    // Bob withdraws first and then claims
    context.switch_account(bob());
    let withdrawn_bob = context.withdraw(&bob(), &product.id);
    let claimed_bob = context.claim_total(&bob());

    assert_eq!(context.claim_total(&bob()), 0);

    assert_eq!(claimed_alice, claimed_bob);
    assert_eq!(withdrawn_alice, withdrawn_bob);

    // All jars were closed and deleted after full withdraw and claim
    assert!(context.contract().get_jars_for_account(alice()).is_empty());
    assert!(context.contract().get_jars_for_account(bob()).is_empty());
}

#[test]
fn revert_scores_on_failed_claim() {
    set_test_log_events(false);

    let term_in_days = 10;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = ProductV2 {
        id: "score_product".to_string(),
        cap: Cap {
            min: 0,
            max: 1_000_000_000_000_000,
        },
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms,
            base_apy: Apy::Constant(UDecimal::zero()),
            score_cap: 20_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    let mut context = Context::new(admin()).with_products(&[product.clone()]).with_jars(
        &alice(),
        &[(product.id.clone(), JarV2::new().with_deposit(0, 100_000_000))],
    );
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));

    for day in 0..=term_in_days {
        context.set_block_timestamp_in_days(day);

        context.record_score(&alice(), (day * MS_IN_DAY).into(), 500);
        if day > 1 {
            context.record_score(&alice(), ((day - 1) * MS_IN_DAY).into(), 1000);
        }

        // Clear accounts cache to test deserialization
        if day == 3 {
            context.contract().accounts_v2.flush();
            context.contract().accounts_v2 = LookupMap::new(StorageKey::AccountsV2);
        }

        // Normal claim. Score should change:
        if day == 4 {
            assert_eq!(context.score(&alice()).scores(), (500, 1000));
            assert_ne!(context.claim_total(&alice()), 0);
            assert_eq!(context.score(&alice()).scores(), (500, 0));
        }

        // Failed claim. Score should stay the same:
        if day == 8 {
            set_test_future_success(false);
            assert_eq!(context.score(&alice()).scores(), (500, 1000));
            assert_eq!(context.claim_total(&alice()), 0);
            assert_eq!(context.score(&alice()).scores(), (500, 1000));
        }
    }
}

impl Context {
    fn interest(&self, account_id: &AccountId, product_id: &ProductId) -> TokenAmount {
        let contract = self.contract();
        let product = &contract.get_product(product_id);
        let account = contract.get_account(account_id);
        let jar = account.get_jar(product_id);

        product.terms.get_interest(account, jar, self.now()).0
    }

    fn jar(&self, account_id: &AccountId, product_id: &ProductId) -> JarV2 {
        let contract = self.contract();
        let account = contract.get_account(account_id);

        account.get_jar(product_id).clone()
    }

    fn claim_total(&mut self, account_id: &AccountId) -> TokenAmount {
        self.switch_account(account_id);
        let PromiseOrValue::Value(claim_result) = self.contract().claim_total(None) else {
            panic!("Expected value");
        };

        claim_result.get_total().0
    }

    fn record_score(&mut self, account_id: &AccountId, time: UTC, score: Score) {
        self.switch_account(admin());
        self.contract()
            .record_score(vec![(account_id.clone(), vec![(score, time)])]);
    }

    pub(crate) fn withdraw(&mut self, account_id: &AccountId, product_id: &ProductId) -> WithdrawView {
        self.switch_account(account_id);
        let result = self.contract().withdraw(product_id.clone());

        match result {
            PromiseOrValue::Promise(_) => {
                panic!("Expected value");
            }
            PromiseOrValue::Value(value) => value,
        }
    }

    fn score(&self, account_id: &AccountId) -> AccountScore {
        self.contract().get_account(account_id).score
    }
}
