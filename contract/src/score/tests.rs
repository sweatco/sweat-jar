#![cfg(test)]

use near_sdk::{test_utils::test_env::alice, AccountId, PromiseOrValue};
use sweat_jar_model::{
    api::{ClaimApi, ScoreApi},
    ProductId, Timezone, TokenAmount, UDecimal, MS_IN_DAY, UTC,
};

use crate::{
    common::{test_data::set_test_log_events, tests::Context},
    jar::model::JarV2,
    product::model::{
        v2::{Apy, FixedProductTerms, InterestCalculator, ScoreBasedProductTerms, Terms},
        ProductV2,
    },
    score::AccountScore,
    test_utils::admin,
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
    const TERM_IN_DAYS: u64 = 365;
    const TERM_IN_MS: u64 = TERM_IN_DAYS * MS_IN_DAY;
    const HALF_PERIOD: u64 = TERM_IN_DAYS / 2;

    set_test_log_events(false);

    let regular_product = ProductV2::new()
        .with_id("regular_product".into())
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: TERM_IN_MS,
            apy: Apy::Constant(UDecimal::new(12000, 5)),
        }));
    let score_product = ProductV2::new()
        .with_id("score_product".into())
        .with_terms(Terms::ScoreBased(ScoreBasedProductTerms {
            score_cap: 12_000,
            base_apy: Apy::Constant(UDecimal::zero()),
            lockup_term: TERM_IN_MS,
        }));

    let regular_product_jar = JarV2::new().with_deposit(0, 100);
    let score_product_jar = JarV2::new().with_deposit(0, 100);

    let mut context = Context::new(admin())
        .with_products(&[regular_product.clone(), score_product.clone()])
        .with_jars(
            &alice(),
            &[
                (regular_product.id.clone(), regular_product_jar),
                (score_product.id.clone(), score_product_jar),
            ],
        );
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(3));

    // Difference of 1 is okay because the missing yoctosweat is stored in claim remainder
    // and will eventually be added to total claimed balance
    fn compare_interest(context: &Context, regular_product_id: &ProductId, score_product_id: &ProductId) {
        let regular_interest = context.interest(&alice(), regular_product_id);
        let score_interest = context.interest(&alice(), score_product_id);
        let diff = regular_interest.abs_diff(score_interest);

        println!(
            "@@ compare interests: regular = {}, score = {}, diff = {}",
            regular_interest, score_interest, diff
        );

        assert!(diff <= 1, "Diff is too big {diff}");
    }

    let mut total_claimed = 0;

    for day in 0..TERM_IN_DAYS {
        let now = day * MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);

        context.switch_account(admin());
        context
            .contract()
            .record_score(vec![(alice(), vec![(20_000, UTC(day * MS_IN_DAY))])]);

        compare_interest(&context, &regular_product.id, &score_product.id);

        if day == HALF_PERIOD {
            let jar_interest = context.interest(&alice(), &regular_product.id);
            let score_interest = context.interest(&alice(), &score_product.id);

            let claimed = context.claim_total(&alice());

            total_claimed += claimed;

            assert_eq!(claimed, jar_interest + score_interest);
        }
    }

    assert_eq!(
        context.jar(&alice(), &regular_product.id).cache.unwrap().updated_at,
        HALF_PERIOD * MS_IN_DAY
    );
    assert_eq!(
        context.jar(&alice(), &score_product.id).cache.unwrap().updated_at,
        (TERM_IN_DAYS - 1) * MS_IN_DAY
    );

    context.set_block_timestamp_in_ms(TERM_IN_MS);
    compare_interest(&context, &regular_product.id, &score_product.id);

    total_claimed += context.claim_total(&alice());
    assert_eq!(total_claimed, 24);
}

// #[test]
// fn score_jar_claim_often_vs_claim_at_the_end() {
//     const ALICE_JAR: JarId = 0;
//     const BOB_JAR: JarId = 1;
//
//     set_test_log_events(false);
//
//     let mut ctx = TestBuilder::new()
//         .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
//         .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
//         .jar(
//             BOB_JAR,
//             [JarField::Account(bob()), JarField::Timezone(Timezone::hour_shift(0))],
//         )
//         .build();
//
//     fn update_and_check(day: u64, ctx: &mut Context, total_claimed_bob: &mut u128) {
//         let score: Score = (0..1000).fake();
//
//         ctx.switch_account(admin());
//         ctx.record_score(UTC(day * MS_IN_DAY), score, alice());
//         ctx.record_score(UTC(day * MS_IN_DAY), score, bob());
//
//         if day > 1 {
//             ctx.switch_account(admin());
//             ctx.record_score(UTC((day - 1) * MS_IN_DAY), score, alice());
//             ctx.record_score(UTC((day - 1) * MS_IN_DAY), score, bob());
//         }
//
//         *total_claimed_bob += ctx.claim_total(bob());
//         assert_eq!(ctx.interest(ALICE_JAR), *total_claimed_bob, "{day}");
//     }
//
//     let mut total_claimed_bob: u128 = 0;
//
//     // Update each hour for 10 days
//     for hour in 0..(24 * 10) {
//         ctx.set_block_timestamp_in_hours(hour);
//         update_and_check(hour / 24, &mut ctx, &mut total_claimed_bob);
//     }
//
//     // Update each day until 100 days has passed
//     for day in 10..100 {
//         ctx.set_block_timestamp_in_days(day);
//         update_and_check(day, &mut ctx, &mut total_claimed_bob);
//     }
//
//     total_claimed_bob += ctx.claim_total(bob());
//
//     assert_eq!(ctx.interest(ALICE_JAR), total_claimed_bob);
//     assert_eq!(ctx.claim_total(alice()), total_claimed_bob);
//
//     assert_eq!(ctx.jar(ALICE_JAR).cache.unwrap().updated_at, MS_IN_DAY * 99);
// }
//
// #[test]
// fn interest_does_not_increase_with_no_steps() {
//     const ALICE_JAR: JarId = 0;
//
//     set_test_log_events(false);
//
//     let mut ctx = TestBuilder::new()
//         .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
//         .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
//         .build();
//
//     ctx.set_block_timestamp_in_days(5);
//
//     ctx.record_score(UTC(5 * MS_IN_DAY), 1000, alice());
//
//     assert_eq!(ctx.interest(ALICE_JAR), 0);
//
//     ctx.set_block_timestamp_in_days(6);
//
//     let interest_for_one_day = ctx.interest(ALICE_JAR);
//     assert_ne!(interest_for_one_day, 0);
//
//     ctx.set_block_timestamp_in_days(7);
//     assert_eq!(interest_for_one_day, ctx.interest(ALICE_JAR));
//
//     ctx.set_block_timestamp_in_days(50);
//     assert_eq!(interest_for_one_day, ctx.interest(ALICE_JAR));
//
//     ctx.set_block_timestamp_in_days(100);
//     assert_eq!(interest_for_one_day, ctx.interest(ALICE_JAR));
// }
//
// #[test]
// fn withdraw_score_jar() {
//     const ALICE_JAR: JarId = 0;
//     const BOB_JAR: JarId = 1;
//
//     set_test_log_events(false);
//
//     let mut ctx = TestBuilder::new()
//         .product(SCORE_PRODUCT, [APY(0), TermDays(7), ScoreCap(20_000)])
//         .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
//         .jar(
//             BOB_JAR,
//             [JarField::Account(bob()), JarField::Timezone(Timezone::hour_shift(0))],
//         )
//         .build();
//
//     for i in 0..=10 {
//         ctx.set_block_timestamp_in_days(i);
//
//         ctx.record_score((i * MS_IN_DAY).into(), 1000, alice());
//         ctx.record_score((i * MS_IN_DAY).into(), 1000, bob());
//
//         if i == 5 {
//             let claimed_alice = ctx.claim_total(alice());
//             let claimed_bob = ctx.claim_total(bob());
//             assert_eq!(claimed_alice, claimed_bob);
//         }
//     }
//
//     // Alice claims first and then withdraws
//     ctx.switch_account(alice());
//     let claimed_alice = ctx.claim_total(alice());
//     let withdrawn_alice = ctx
//         .contract()
//         .withdraw(ALICE_JAR.into(), None)
//         .unwrap()
//         .withdrawn_amount
//         .0;
//
//     assert_eq!(ctx.claim_total(alice()), 0);
//
//     // Bob withdraws first and then claims
//     ctx.switch_account(bob());
//     let withdrawn_bob = ctx
//         .contract()
//         .withdraw(BOB_JAR.into(), None)
//         .unwrap()
//         .withdrawn_amount
//         .0;
//     let claimed_bob = ctx.claim_total(bob());
//
//     assert_eq!(ctx.claim_total(bob()), 0);
//
//     assert_eq!(claimed_alice, claimed_bob);
//     assert_eq!(withdrawn_alice, withdrawn_bob);
//
//     // All jars were closed and deleted after full withdraw and claim
//     assert!(ctx.contract().account_jars(&alice()).is_empty());
//     assert!(ctx.contract().account_jars(&bob()).is_empty());
// }
//
// #[test]
// fn revert_scores_on_failed_claim() {
//     const ALICE_JAR: JarId = 0;
//
//     set_test_log_events(false);
//
//     let mut ctx = TestBuilder::new()
//         .product(SCORE_PRODUCT, [APY(0), TermDays(10), ScoreCap(20_000)])
//         .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
//         .build();
//
//     for day in 0..=10 {
//         ctx.set_block_timestamp_in_days(day);
//
//         ctx.record_score((day * MS_IN_DAY).into(), 500, alice());
//         if day > 1 {
//             ctx.record_score(((day - 1) * MS_IN_DAY).into(), 1000, alice());
//         }
//
//         // Clear accounts cache to test deserialization
//         if day == 3 {
//             ctx.contract().accounts.flush();
//             ctx.contract().accounts = LookupMap::new(StorageKey::AccountsVersioned);
//         }
//
//         // Normal claim. Score should change:
//         if day == 4 {
//             assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 1000));
//             assert_ne!(ctx.claim_total(alice()), 0);
//             assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 0));
//         }
//
//         // Failed claim. Score should stay the same:
//         if day == 8 {
//             set_test_future_success(false);
//             assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 1000));
//             assert_eq!(ctx.claim_total(alice()), 0);
//             assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 1000));
//         }
//     }
// }

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
}
