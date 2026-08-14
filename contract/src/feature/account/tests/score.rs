#![cfg(test)]

use near_sdk::{json_types::U128, store::LookupMap, AccountId, PromiseOrValue, Timestamp};
use rstest::{fixture, rstest};
use sweat_jar_model::{
    api::{AccountApi, ClaimApi, WithdrawApi},
    data::{
        deposit::DepositTicket,
        jar::Jar,
        product::{Product, ProductId},
        withdraw::WithdrawView,
    },
    interest::InterestCalculator,
    AccountScore, Score, Timezone, TokenAmount, MS_IN_DAY, MS_IN_HOUR, MS_IN_YEAR, UTC,
};

use crate::{
    common::{
        env::test_env_ext,
        testing::{
            accounts::{admin, alice, bob},
            Context, TokenUtils,
        },
    },
    feature::{account::model::test_utils::jar, product::model::test_utils::*},
    StorageKey,
};

mod score_tests {
    use super::*;

    #[rstest]
    #[should_panic(expected = "Insufficient permissions")]
    fn record_score_by_non_oracle(admin: AccountId) {
        let mut context = Context::new(admin);

        context.switch_account(alice());
        context.contract().record_score(vec![(alice(), vec![(100, 0.into())])]);
    }

    #[rstest]
    fn interest_does_not_increase_with_no_score(
        admin: AccountId,
        alice: AccountId,
        #[from(product_steps_365d_20000_score_cap)] product: Product,
        #[with(vec![(0, 100_000_000.to_otto())])] jar: Jar,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut context = Context::new(admin)
            .with_products(&[product.clone()])
            .with_latest_account(&alice, &[(product.id.clone(), jar)]);
        context.contract().get_account_mut(&alice).score = AccountScore::default();
        context.contract().get_account_mut(&alice).timezone = Timezone::hour_shift(0);

        context.set_block_timestamp_in_days(5);

        context.record_score(&alice, UTC(5 * MS_IN_DAY), 1000);

        assert_eq!(context.interest(&alice, &product.id), 0);

        context.set_block_timestamp_in_days(7);

        let interest_for_one_day = context.interest(&alice, &product.id);
        assert_ne!(interest_for_one_day, 0);

        context.set_block_timestamp_in_days(8);
        assert_eq!(interest_for_one_day, context.interest(&alice, &product.id));

        context.set_block_timestamp_in_days(50);
        assert_eq!(interest_for_one_day, context.interest(&alice, &product.id));

        context.set_block_timestamp_in_days(100);
        assert_eq!(interest_for_one_day, context.interest(&alice, &product.id));
    }

    #[rstest]
    fn withdraw_score_jar(
        admin: AccountId,
        alice: AccountId,
        bob: AccountId,
        #[from(product_7_days_20_cap_score_based)] product: Product,
        #[with(vec![(0, 100)])] jar: Jar,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut context = Context::new(admin)
            .with_products(&[product.clone()])
            .with_latest_account(&alice, &[(product.id.clone(), jar.clone())])
            .with_latest_account(&bob, &[(product.id.clone(), jar.clone())]);
        context.contract().get_account_mut(&alice).score = AccountScore::default();
        context.contract().get_account_mut(&alice).timezone = Timezone::hour_shift(0);
        context.contract().get_account_mut(&bob).score = AccountScore::default();
        context.contract().get_account_mut(&bob).timezone = Timezone::hour_shift(0);

        for i in 0..=10 {
            context.set_block_timestamp_in_days(i);

            context.record_score(&alice, (i * MS_IN_DAY).into(), 1000);
            context.record_score(&bob, (i * MS_IN_DAY).into(), 1000);

            if i == 5 {
                let claimed_alice = context.claim_total(&alice);
                let claimed_bob = context.claim_total(&bob);
                assert_eq!(claimed_alice, claimed_bob);
            }
        }

        // Alice claims first and then withdraws
        let claimed_alice = context.claim_total(&alice);
        let withdrawn_alice = context.withdraw(&alice, &product.id);

        assert_eq!(context.claim_total(&alice), 0);

        // Bob withdraws first and then claims
        context.switch_account(bob.clone());
        let withdrawn_bob = context.withdraw(&bob, &product.id);
        let claimed_bob = context.claim_total(&bob);

        assert_eq!(context.claim_total(&bob), 0);

        assert_eq!(claimed_alice, claimed_bob);
        assert_eq!(withdrawn_alice, withdrawn_bob);

        // All jars were closed and deleted after full withdraw and claim
        assert!(context.contract().get_jars_for_account(alice.clone()).is_empty());
        assert!(context.contract().get_jars_for_account(bob.clone()).is_empty());

        assert!(context.contract().get_jars_for_account_detailed(&alice).is_empty());
        assert!(context.contract().get_jars_for_account_detailed(&bob).is_empty());
    }

    #[rstest]
    fn revert_scores_on_failed_claim(
        admin: AccountId,
        alice: AccountId,
        #[from(product_10_days_20_cap_score_based)] product: Product,
        #[with(vec![(0, 100_000_000)])] jar: Jar,
    ) {
        test_env_ext::set_test_log_events(false);

        let term_in_days = product.terms.get_lockup_term().unwrap() / MS_IN_DAY;

        let mut context = Context::new(admin)
            .with_products(&[product.clone()])
            .with_latest_account(&alice, &[(product.id.clone(), jar)]);
        context.contract().get_account_mut(&alice).score = AccountScore::default();
        context.contract().get_account_mut(&alice).timezone = Timezone::hour_shift(0);

        for day in 0..=term_in_days {
            context.set_block_timestamp_in_days(day);

            context.record_score(&alice, (day * MS_IN_DAY).into(), 500);
            if day > 1 {
                context.record_score(&alice, ((day - 1) * MS_IN_DAY).into(), 1000);
            }

            // Clear accounts cache to test deserialization
            if day == 3 {
                context.contract().accounts.flush();
                context.contract().accounts = LookupMap::new(StorageKey::Accounts);
            }

            // Normal claim. Score shouldn't change:
            if day == 4 {
                assert_eq!(context.score(&alice).scores(), (500, 1500));
                assert_ne!(context.claim_total(&alice), 0);
                assert_eq!(context.score(&alice).scores(), (500, 1500));
            }

            // Failed claim. Score should stay the same:
            if day == 8 {
                test_env_ext::set_test_future_success(false);
                assert_eq!(context.score(&alice).scores(), (500, 1500));
                assert_eq!(context.claim_total(&alice), 0);
                assert_eq!(context.score(&alice).scores(), (500, 1500));
            }
        }
    }

    #[rstest]
    #[should_panic(expected = "Timezone is not set for account 'alice.near'")]
    fn test_score_recording_before_before_tizone_is_set(
        admin: AccountId,
        alice: AccountId,
        #[from(product_10_days_20_cap_score_based)] product: Product,
        #[with(vec![(0, 100)])] jar: Jar,
    ) {
        let mut context = Context::new(admin.clone())
            .with_products(&[product.clone()])
            .with_latest_account(&alice, &[(product.id.clone(), jar)]);

        context.switch_account_to_operator();
        context
            .contract()
            .record_score(vec![(alice.clone(), vec![(0, 10_000.into())])]);
    }

    #[rstest]
    fn test_steps_history(
        admin: AccountId,
        alice: AccountId,
        #[values(1_729_692_817_027)] base_time: Timestamp,
        #[from(product_10_days_20_cap_score_based)] product: Product,
        #[with(vec![(base_time, 100)])] jar: Jar,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone())
            .with_products(&[product.clone()])
            .with_latest_account(&alice, &[(product.id.clone(), jar)]);
        ctx.contract().get_account_mut(&alice).timezone = Timezone::hour_shift(4);

        let check_score_interest = |ctx: &Context, val: u128| {
            assert_eq!(ctx.contract().get_score(alice.clone()), Some(U128(val)));
        };

        ctx.set_block_timestamp_in_ms(base_time);

        check_score_interest(&ctx, 0);

        ctx.record_score(&alice, UTC(base_time - MS_IN_DAY), 8245);

        check_score_interest(&ctx, 8245);

        ctx.set_block_timestamp_in_ms(base_time + MS_IN_DAY);

        check_score_interest(&ctx, 0);

        ctx.set_block_timestamp_in_ms(base_time + MS_IN_DAY * 10);

        check_score_interest(&ctx, 0);

        ctx.record_score(&alice, UTC(base_time + MS_IN_DAY * 10), 10000);
        ctx.record_score(&alice, UTC(base_time + MS_IN_DAY * 10), 101);
        ctx.record_score(&alice, UTC(base_time + MS_IN_DAY * 9), 9000);
        ctx.record_score(&alice, UTC(base_time + MS_IN_DAY * 9), 90);

        check_score_interest(&ctx, 9090);

        ctx.set_block_timestamp_in_ms(base_time + MS_IN_DAY * 11);

        check_score_interest(&ctx, 10101);

        ctx.set_block_timestamp_in_ms(base_time + MS_IN_DAY * 12);

        check_score_interest(&ctx, 0);
    }

    #[rstest]
    fn record_max_score(
        admin: AccountId,
        alice: AccountId,
        #[from(product_10_days_20_cap_score_based)] product: Product,
        #[with(vec![(0, 100)])] jar: Jar,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone())
            .with_products(&[product.clone()])
            .with_latest_account(&alice, &[(product.id.clone(), jar)]);
        ctx.contract().get_account_mut(&alice).timezone = Timezone::hour_shift(4);

        ctx.record_score(&alice, UTC(0), 25000);
        ctx.record_score(&alice, UTC(0), 25000);
        ctx.record_score(&alice, UTC(0), 25000);
        ctx.record_score(&alice, UTC(0), 25000);

        ctx.set_block_timestamp_in_days(1);

        assert_eq!(ctx.contract().get_score(alice).unwrap().0, 65535);
    }

    #[rstest]
    fn claim_when_there_were_no_walkchains_for_some_time(
        admin: AccountId,
        alice: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let ticket = DepositTicket {
            product_id: product.id.clone(),
            valid_until: (1_733_139_450_015 + MS_IN_YEAR).into(),
            timezone: Some(Timezone::hour_shift(0)),
        };

        ctx.switch_account(admin.clone());
        ctx.set_block_timestamp_in_ms(1_732_653_318_018 - MS_IN_DAY);
        ctx.contract().deposit(alice.clone(), ticket.clone(), 0, None);

        ctx.set_block_timestamp_in_ms(1_732_653_318_018);
        ctx.contract()
            .record_score(vec![(alice.clone(), vec![(15100, 1_732_653_318_018.into())])]);

        ctx.set_block_timestamp_in_ms(1_733_139_450_015);
        ctx.contract()
            .deposit(alice.clone(), ticket, 100_000_000.to_otto(), None);

        ctx.set_block_timestamp_in_ms(1_733_140_384_365); // Mon Dec 02 2024 11:53:04

        assert_eq!(0, ctx.contract().get_total_interest(alice.clone()).amount.total.0);
    }

    #[rstest]
    fn record_multiple_scores_exceeding_cap_for_score_based_product(
        admin: AccountId,
        alice: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let star_time = 1_761_955_200_000;
        ctx.set_block_timestamp_in_ms(star_time);

        ctx.contract().get_or_create_account_mut(&alice).deposit(
            &product.id,
            365_000_000_000_000_000_000,
            star_time.into(),
        );
        ctx.contract()
            .get_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());

        ctx.set_block_timestamp_in_ms(star_time);

        let record_timestamp = star_time - 5 * MS_IN_HOUR;

        // STEP 1: record score_1: score_1 < score_cap
        {
            ctx.switch_account_to_operator();
            ctx.contract()
                .record_score(vec![(alice.clone(), vec![(15_000, record_timestamp.into())])]);

            ctx.set_block_timestamp_in_ms(star_time + MS_IN_DAY / 2);

            ctx.switch_account(alice.clone());
            assert_eq!(75_000_000_000_000_000, ctx.claim_total(&alice));
            assert_eq!(0, ctx.contract().get_total_interest(alice.clone()).amount.total.0);
        }

        // STEP 2: record score_2: (score_1 + score_2) > score_cap
        {
            ctx.switch_account_to_operator();
            ctx.contract()
                .record_score(vec![(alice.clone(), vec![(15_000, record_timestamp.into())])]);

            assert_eq!(0, ctx.contract().get_total_interest(alice.clone()).amount.total.0);

            ctx.set_block_timestamp_in_ms(star_time + MS_IN_DAY);

            ctx.switch_account(alice.clone());
            assert_eq!(90_000_000_000_000_000, ctx.claim_total(&alice));
        }
    }

    #[rstest]
    fn record_multiple_scores_exceeding_cap_for_tiered_score_based_product(
        admin: AccountId,
        alice: AccountId,
        #[from(tiered_score_based_product)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let star_time = 1_761_955_200_000;
        ctx.set_block_timestamp_in_ms(star_time);

        ctx.contract().get_or_create_account_mut(&alice).deposit(
            &product.id,
            365_000_000_000_000_000_000,
            star_time.into(),
        );
        ctx.contract()
            .get_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());

        let recording_time = star_time - 5 * MS_IN_HOUR;

        // STEP 1: record score_1: score_1 < score_cap
        {
            ctx.switch_account_to_operator();
            ctx.contract()
                .record_score(vec![(alice.clone(), vec![(7_000, recording_time.into())])]);

            ctx.set_block_timestamp_in_ms(star_time + 12 * MS_IN_HOUR);
            ctx.switch_account(alice.clone());
            assert_eq!(35_000_000_000_000_000, ctx.claim_total(&alice));
        }

        // STEP 2: record score_2: (score_1 + score_2) > score_cap
        {
            ctx.switch_account_to_operator();
            ctx.contract()
                .record_score(vec![(alice.clone(), vec![(5_000, recording_time.into())])]);

            ctx.set_block_timestamp_in_ms(star_time + 24 * MS_IN_HOUR);
            ctx.switch_account(alice.clone());
            assert_eq!(50_000_000_000_000_000, ctx.claim_total(&alice));
        }
    }

    #[rstest]
    fn apply_finalized_to_new_deposit_after_claim_from_score_based_product(
        admin: AccountId,
        alice: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let star_time = 1_761_955_200_000;
        ctx.set_block_timestamp_in_ms(star_time);

        ctx.contract()
            .get_or_create_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());
        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, 365_000_000_000_000_000_000, star_time.into());

        let mut action_time = star_time + MS_IN_DAY;
        ctx.set_block_timestamp_in_ms(action_time);

        // STEP 1: record score and claim
        {
            ctx.switch_account_to_operator();
            ctx.contract().record_score(vec![(
                alice.clone(),
                vec![(5_000, (action_time - 5 * MS_IN_HOUR).into())],
            )]);

            action_time += 6 * MS_IN_HOUR;
            ctx.set_block_timestamp_in_ms(action_time);

            ctx.switch_account(alice.clone());
            assert_eq!(12_500_000_000_000_000, ctx.claim_total(&alice));
        }

        action_time += MS_IN_HOUR;
        ctx.set_block_timestamp_in_ms(action_time);

        // STEP 2: new deposit and claim
        {
            ctx.contract().get_account_mut(&alice).deposit(
                &product.id,
                365_000_000_000_000_000_000,
                action_time.into(),
            );

            ctx.switch_account(alice.clone());
            assert_eq!(2_083_333_333_333_333, ctx.claim_total(&alice));
        }
    }

    #[rstest]
    fn apply_finalized_to_new_deposit_after_claim_from_tiered_score_based_product(
        admin: AccountId,
        alice: AccountId,
        #[from(tiered_score_based_product)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let start_time = 1_761_955_200_000;
        ctx.set_block_timestamp_in_ms(start_time);

        ctx.contract()
            .get_or_create_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());
        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, 365_000_000_000_000_000_000, start_time.into());

        let recording_time = start_time - 5 * MS_IN_HOUR;

        // STEP 1: record score and claim
        {
            ctx.switch_account_to_operator();
            ctx.contract()
                .record_score(vec![(alice.clone(), vec![(5_000, recording_time.into())])]);
            ctx.contract()
                .apply_booster(vec![alice.clone()], 30_000, recording_time.into());

            ctx.set_block_timestamp_in_ms(start_time + 12 * MS_IN_HOUR);
            ctx.switch_account(alice.clone());
            assert_eq!(175_000_000_000_000_000, ctx.claim_total(&alice));
        }

        // STEP 2: new deposit and claim
        {
            ctx.contract().get_account_mut(&alice).deposit(
                &product.id,
                365_000_000_000_000_000_000,
                (start_time + 12 * MS_IN_HOUR).into(),
            );

            ctx.set_block_timestamp_in_ms(start_time + 24 * MS_IN_HOUR);
            ctx.switch_account(alice.clone());
            assert_eq!(350_000_000_000_000_000, ctx.claim_total(&alice));
        }
    }

    /// Tests that settle_interest correctly adds (not multiplies) remainder values.
    /// This catches mutation: replace + with * in `let remainder = jar.claim_remainder + remainder`
    #[rstest]
    fn settle_interest_accumulates_remainder_correctly(
        admin: AccountId,
        alice: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let start_time = MS_IN_DAY * 100;
        ctx.set_block_timestamp_in_ms(start_time);

        ctx.contract()
            .get_or_create_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());

        // Small deposit to generate non-trivial remainders
        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, 1_000_000, start_time.into());

        // Record score for day 0
        ctx.switch_account_to_operator();
        ctx.record_score(&alice, (start_time - 6 * MS_IN_HOUR).into(), 10_000);

        // Move to day 1 and claim - this will set claim_remainder
        ctx.set_block_timestamp_in_ms(start_time + MS_IN_DAY);
        let first_claim = ctx.claim_total(&alice);

        let remainder_after_first_claim = ctx.contract().get_account(&alice).get_jar(&product.id).claim_remainder;

        // Record score for day 1
        ctx.record_score(&alice, (start_time + MS_IN_DAY - 6 * MS_IN_HOUR).into(), 10_000);

        // Move to day 2 - settle_interest will be called during claim
        // At this point: claim_remainder = remainder_after_first_claim (non-zero)
        // and new remainder will be calculated from get_settled_interest
        ctx.set_block_timestamp_in_ms(start_time + 2 * MS_IN_DAY);

        // Directly test settle_interest accumulation
        ctx.contract().settle_interest(&alice);

        // After settle_interest, the remainder should be: old_remainder + new_remainder
        // If multiplication was used, it would be: old_remainder * new_remainder (different value)
        let remainder_after_settle = ctx.contract().get_account(&alice).get_jar(&product.id).claim_remainder;

        // The remainder should have accumulated (added), not multiplied
        // With same conditions each day, remainder should roughly double (addition)
        // With multiplication: if both are ~X, then X*X >> X+X which would overflow u64

        // First, verify we have non-zero remainder to test with
        assert!(
            remainder_after_first_claim > 0,
            "Test setup error: first claim should produce non-zero remainder"
        );

        // With addition: result ≈ 2 * remainder_after_first_claim
        // With multiplication: result would overflow or be wildly different
        // We check that the result is close to 2x (within 50% margin)
        let expected_sum = remainder_after_first_claim * 2;
        assert!(
            remainder_after_settle >= expected_sum / 2 && remainder_after_settle <= expected_sum * 2,
            "Remainder should accumulate via addition, not multiplication. \
             First remainder: {}, After settle: {}, Expected ~{}",
            remainder_after_first_claim,
            remainder_after_settle,
            expected_sum
        );

        let second_claim = ctx.claim_total(&alice);

        // Both claims should be roughly equal (same score, same duration)
        assert!(first_claim > 0, "First claim should be non-zero");
        assert!(second_claim > 0, "Second claim should be non-zero");
        // The claims should be approximately equal (within 1 unit due to remainder accumulation)
        assert!(
            (first_claim as i128 - second_claim as i128).abs() <= 1,
            "Claims should be approximately equal: first={}, second={}",
            first_claim,
            second_claim
        );
    }

    /// Tests that get_settled_interest correctly calculates day offsets using multiplication.
    /// This catches mutation: replace * with / in `(i as u64) * ms_in_day()`
    ///
    /// Key insight: With `/`, the mutation causes `i / ms_in_day() = 0` for all reasonable i values,
    /// so all loop iterations use the SAME day_start. This causes duplicate interest calculation
    /// for one day instead of calculating interest for different days.
    ///
    /// IMPORTANT: The code uses `adjust_relative()` which shifts timestamps back by 1 day.
    /// So a deposit at day 1 + 12h has deposit_created_at_relative = day 0 + 12h.
    ///
    /// To catch this mutation, we create deposit at day 1 + 12h so that (after adjust_relative):
    /// - With `*`: day 0 gets 12h interest, day 1 gets 24h interest → ~1.5 days total
    /// - With `/`: both iterations calculate for day 1 (24h each) → ~2 days total (WRONG)
    #[rstest]
    fn get_settled_interest_calculates_multiple_days_correctly(
        admin: AccountId,
        alice: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        // Day 0 starts at this time
        let day0_start = MS_IN_DAY * 100;
        let day1_start = day0_start + MS_IN_DAY;

        // Record score for day 0 first (need to do this before deposit for timezone setup)
        ctx.set_block_timestamp_in_ms(day0_start + 6 * MS_IN_HOUR);
        ctx.contract()
            .get_or_create_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());

        ctx.switch_account_to_operator();
        ctx.record_score(&alice, (day0_start + 6 * MS_IN_HOUR).into(), 10_000);

        // Create deposit at MID-DAY of day 1 (12 hours into day 1)
        // After adjust_relative (-1 day), this becomes day 0 + 12h
        // This means:
        // - Day 0 period: deposit_created_at_relative = day0 + 12h, so term = 12h (partial)
        // - Day 1 period: deposit_created_at_relative = day0 + 12h < day1 start, so term = 24h (full)
        let deposit_time = day1_start + 12 * MS_IN_HOUR;
        ctx.set_block_timestamp_in_ms(deposit_time);

        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, 365_000_000_000_000_000_000, deposit_time.into());

        // Move to day 1 + 18h and record score for day 1
        ctx.set_block_timestamp_in_ms(day1_start + 18 * MS_IN_HOUR);
        ctx.record_score(&alice, (day1_start + 6 * MS_IN_HOUR).into(), 10_000);

        // Move forward to day 4 (more than 1 day since last score update triggers Greater branch)
        ctx.set_block_timestamp_in_ms(day0_start + 4 * MS_IN_DAY);

        let interest = ctx.interest(&alice, &product.id);

        // With 10k score = 10% APY, 365_000 SWEAT deposit:
        // - Day 0: deposit_created_at_relative = day0+12h, so 12h interest → 50 SWEAT
        // - Day 1: deposit_created_at_relative < day1 start, so full 24h → 100 SWEAT
        // - Total with correct `*`: ~150 SWEAT (1.5 days)
        //
        // With mutation `/`:
        // - Both iterations use day 1's time range (day_start = day1 for both i=0 and i=1)
        // - For both: deposit_created_at_relative = day0+12h < day1 start
        // - Each calculates full 24 hours → 100 SWEAT each
        // - Total: ~200 SWEAT (2 days - WRONG, duplicate!)
        //
        // We check that interest is around 150 SWEAT, NOT 200 SWEAT
        let expected_interest = 150_000_000_000_000_000u128; // ~150 SWEAT

        assert!(
            interest >= expected_interest - 20_000_000_000_000_000
                && interest <= expected_interest + 20_000_000_000_000_000,
            "Interest should be ~150 SWEAT (1.5 days: 12h day 0 + 24h day 1). Got: {}. \
             If this is ~200 SWEAT, the day calculation is using `/` instead of `*`, \
             causing duplicate calculation for day 1.",
            interest
        );
    }

    /// Tests that interest from multiple deposits is summed (not multiplied) in fold.
    /// This catches mutation: replace + with * in `(acc.0 + interest, acc.1 + remainder)`
    #[rstest]
    fn get_settled_interest_sums_multiple_deposits(
        admin: AccountId,
        alice: AccountId,
        bob: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let start_time = MS_IN_DAY * 100;
        ctx.set_block_timestamp_in_ms(start_time);

        // Setup Alice with two separate deposits
        ctx.contract()
            .get_or_create_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());

        let deposit_amount = 365_000_000_000_000_000_000u128;
        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, deposit_amount, start_time.into());
        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, deposit_amount, start_time.into());

        // Setup Bob with single deposit of double amount
        ctx.contract()
            .get_or_create_account_mut(&bob)
            .try_set_timezone(Timezone::new(0).into());

        ctx.contract()
            .get_account_mut(&bob)
            .deposit(&product.id, deposit_amount * 2, start_time.into());

        // Record same score for both
        ctx.switch_account_to_operator();
        ctx.record_score(&alice, (start_time - 6 * MS_IN_HOUR).into(), 10_000);
        ctx.record_score(&bob, (start_time - 6 * MS_IN_HOUR).into(), 10_000);

        // Move to next day and compare interest
        ctx.set_block_timestamp_in_ms(start_time + MS_IN_DAY);
        let interest_two_deposits = ctx.interest(&alice, &product.id);
        let interest_single_deposit = ctx.interest(&bob, &product.id);

        // CRITICAL: Interest must be non-zero!
        // If multiplication was used in fold (0 * interest = 0), all interest would be 0.
        // With 10k score = 10% APY, 365_000 SWEAT * 2 deposits * 10% / 365 = ~200 SWEAT
        assert!(
            interest_two_deposits >= 180_000_000_000_000_000,
            "Interest from two deposits should be ~200 SWEAT (non-zero). Got: {}. \
             If this is 0, the fold is using multiplication instead of addition.",
            interest_two_deposits
        );

        // Two deposits should produce the same interest as one deposit of double amount
        assert_eq!(
            interest_two_deposits, interest_single_deposit,
            "Interest from two deposits ({}) should equal interest from single double deposit ({})",
            interest_two_deposits, interest_single_deposit
        );
    }

    /// Tests that remainders are accumulated (added, not multiplied) across multiple score days.
    /// This catches mutation: replace += with *= in `current_increment.1 += increment.1`
    #[rstest]
    fn get_settled_interest_accumulates_remainder_across_days(
        admin: AccountId,
        alice: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let start_time = MS_IN_DAY * 100;
        ctx.set_block_timestamp_in_ms(start_time);

        ctx.contract()
            .get_or_create_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());

        // Small deposit that generates non-trivial remainders
        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, 7_000_000, start_time.into());

        // Record scores for two days
        ctx.switch_account_to_operator();
        ctx.record_score(&alice, (start_time - 6 * MS_IN_HOUR).into(), 10_000);
        ctx.set_block_timestamp_in_ms(start_time + MS_IN_DAY);
        ctx.record_score(&alice, (start_time + MS_IN_DAY - 6 * MS_IN_HOUR).into(), 10_000);

        // Move forward 3+ days to trigger Greater branch (processes multiple days)
        ctx.set_block_timestamp_in_ms(start_time + 4 * MS_IN_DAY);

        // Directly call get_settled_interest to verify remainder accumulation
        let settled = ctx.contract().get_settled_interest(&alice);
        let (interest, remainder) = settled.get(&product.id).expect("Should have interest for product");

        // With multiplication instead of addition, remainder would be 0 (0 * X = 0)
        // With addition, remainder should be the sum of remainders from both days
        assert!(
            *remainder > 0,
            "Remainder should be non-zero (sum of remainders from multiple days). Got: {}. \
             If this is 0, the accumulation is using multiplication instead of addition.",
            remainder
        );

        // Interest should also be non-zero
        // The interest depends on how many days of score history are processed
        assert!(
            *interest > 0,
            "Interest should be non-zero. Got: {}. \
             If this is 0, the fold accumulation is using multiplication instead of addition.",
            interest
        );
    }
}

mod account_score_tests {
    use near_sdk::env::block_timestamp_ms;
    use sweat_jar_model::{
        convert_to_days_offset,
        data::account::{features::Feature, versioned::AccountVersioned, Account},
        DailyScore, ScoreIncrementProcessor, ScoreIncrements,
    };
    use sweat_jar_primitives::UDecimal;

    use super::*;

    const TIMEZONE: Timezone = Timezone::hour_shift(3);
    const TODAY: u64 = 1_722_234_632_000;

    #[fixture]
    fn increments() -> ScoreIncrements {
        let today: UTC = TODAY.into();

        vec![
            (1_000, today),
            (1_000, (today.0 - (MS_IN_HOUR * 3)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 12)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 25)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 28)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 40)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 45)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 48)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 55)).into()),
            (1_000, (today.0 - (MS_IN_HOUR * 550)).into()),
        ]
    }

    #[fixture]
    fn context(admin: AccountId) -> Context {
        Context::new(admin)
    }

    #[fixture]
    fn daily_score(#[default(0)] value: Score, #[default(0)] booster: Score) -> DailyScore {
        DailyScore { value, booster }
    }

    #[rstest]
    fn test_account_score(
        mut context: Context,
        #[from(product_10_days_20_cap_score_based)] product: Product,
        increments: ScoreIncrements,
    ) {
        let mut now = TODAY;
        context.set_block_timestamp_in_ms(now);

        let segmented_increments = ScoreIncrementProcessor::new(&increments, TIMEZONE).process();
        let normalized_increments = convert_to_days_offset(segmented_increments.valid, TIMEZONE);

        let mut score = AccountScore::default();
        score.update(normalized_increments);

        let account = Account {
            score,
            timezone: TIMEZONE,
            ..Account::default()
        };

        assert_eq!(0.03, product.terms.get_apy(&account).to_f32());

        now += MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);
        assert_eq!(0.02, product.terms.get_apy(&account).to_f32());

        now += MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);
        assert_eq!(0.0, product.terms.get_apy(&account).to_f32());
    }

    #[rstest]
    #[should_panic(expected = "Timestamp from future: Local(1722331832000). Now: Local(1722245432000)")]
    fn steps_from_future(mut context: Context) {
        context.set_block_timestamp_in_ms(TODAY);

        let increments = vec![(1_000, (block_timestamp_ms() + MS_IN_DAY).into())];
        let segmented_increments = ScoreIncrementProcessor::new(&increments, TIMEZONE).process();
        let normalized_increments = convert_to_days_offset(segmented_increments.valid, TIMEZONE);

        let mut account_score = AccountScore::default();
        account_score.update(normalized_increments);
    }

    #[rstest]
    fn updated_on_different_days(mut context: Context) {
        let timezone = Timezone::hour_shift(0);
        let mut score = AccountScore::new(UTC(MS_IN_DAY * 10), [DailyScore::new(1000), DailyScore::new(2000)]);

        context.set_block_timestamp_in_ms(MS_IN_DAY * 10);

        let increments = vec![(6, (MS_IN_DAY * 10).into()), (5, (MS_IN_DAY * 9).into())];
        let segmented_increments = ScoreIncrementProcessor::new(&increments, timezone).process();
        let normalized_increments = convert_to_days_offset(segmented_increments.valid.clone(), timezone);
        score.update(normalized_increments);

        assert_eq!(score.updated_at(), MS_IN_DAY * 10);
        assert_eq!(score.scores(), (1006, 2005));
        assert_eq!(score.get_last_finalized_record(timezone).value, 2005);

        context.set_block_timestamp_in_ms(MS_IN_DAY * 11);
        assert_eq!(score.get_last_finalized_record(timezone).value, 1006);

        context.set_block_timestamp_in_ms(MS_IN_DAY * 12);
        assert_eq!(score.get_last_finalized_record(timezone).value, 0);
    }

    #[rstest]
    fn active_score(mut context: Context) {
        let timezone = Timezone::hour_shift(0);
        let score = AccountScore::new(UTC(MS_IN_DAY * 10), [DailyScore::new(1000), DailyScore::new(2000)]);

        context.set_block_timestamp_in_ms(MS_IN_DAY * 10);

        assert_eq!(score.get_last_finalized_record(timezone).value, 2000);

        context.set_block_timestamp_in_ms(MS_IN_DAY * 11);

        assert_eq!(score.get_last_finalized_record(timezone).value, 1000);

        context.set_block_timestamp_in_ms(MS_IN_DAY * 12);

        assert_eq!(score.get_last_finalized_record(timezone).value, 0);
    }

    #[rstest]
    fn claim_from_tiered_score_jar_with_increased_cap_fearure(
        admin: AccountId,
        alice: AccountId,
        #[from(tiered_score_based_product)] product: Product,
    ) {
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);
        context.switch_account_to_operator();

        context
            .contract()
            .accounts
            .set(alice.clone(), AccountVersioned::new(Account::default()).into());
        context
            .contract()
            .set_feature_enabled(alice.clone(), Feature::IncreasedScoreCap, true);
        context.contract().set_timezone(alice.clone(), 0.into());
        context.contract().deposit(
            alice.clone(),
            DepositTicket {
                product_id: product.id.clone(),
                valid_until: MS_IN_YEAR.into(),
                timezone: Some(Timezone::hour_shift(0)),
            },
            365_000.to_otto(),
            None,
        );

        context.set_block_timestamp_in_ms(0);
        context
            .contract()
            .record_score(vec![(alice.clone(), vec![(25_000, 0.into())])]);

        context.set_block_timestamp_in_ms(MS_IN_DAY);
        context
            .contract()
            .record_score(vec![(alice.clone(), vec![(30_000, MS_IN_DAY.into())])]);

        context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
        let interest = context.contract().get_total_interest(alice.clone());
        assert_eq!(400.to_otto(), interest.amount.total.0);
    }

    #[rstest]
    fn claim_from_tiered_score_jar_with_increased_cap_fearure_disabled_later(
        admin: AccountId,
        alice: AccountId,
        #[from(tiered_score_based_product)] product: Product,
    ) {
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);
        context.switch_account_to_operator();

        context
            .contract()
            .accounts
            .set(alice.clone(), AccountVersioned::new(Account::default()).into());
        context
            .contract()
            .set_feature_enabled(alice.clone(), Feature::IncreasedScoreCap, true);
        context.contract().set_timezone(alice.clone(), 0.into());
        context.contract().deposit(
            alice.clone(),
            DepositTicket {
                product_id: product.id.clone(),
                valid_until: MS_IN_YEAR.into(),
                timezone: Some(Timezone::hour_shift(0)),
            },
            365_000.to_otto(),
            None,
        );

        context.set_block_timestamp_in_ms(0);
        context
            .contract()
            .record_score(vec![(alice.clone(), vec![(25_000, 0.into())])]);

        context.set_block_timestamp_in_ms(2 * MS_IN_DAY);
        context
            .contract()
            .set_feature_enabled(alice.clone(), Feature::IncreasedScoreCap, false);
        context
            .contract()
            .record_score(vec![(alice.clone(), vec![(30_000, MS_IN_DAY.into())])]);

        context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
        let interest = context.contract().get_total_interest(alice.clone());
        assert_eq!(300.to_otto(), interest.amount.total.0);
    }

    #[rstest]
    fn continuous_claim_from_score_based_jar(
        admin: AccountId,
        alice: AccountId,
        #[from(product_7_days_18_cap_score_based)] product: Product,
    ) {
        test_env_ext::set_test_log_events(false);

        let mut ctx = Context::new(admin.clone()).with_products(&[product.clone()]);

        let star_time = 1_761_955_200_000;
        ctx.set_block_timestamp_in_ms(star_time);

        ctx.contract()
            .get_or_create_account_mut(&alice)
            .try_set_timezone(Timezone::new(0).into());
        ctx.contract()
            .get_account_mut(&alice)
            .deposit(&product.id, 365_000_000_000_000_000_000, star_time.into());

        {
            ctx.switch_account_to_operator();
            ctx.record_score(&alice, (star_time - 6 * MS_IN_HOUR).into(), 10_000);

            ctx.set_block_timestamp_in_ms(star_time + 2 * MS_IN_HOUR);
            assert_eq!(8_333_333_333_333_333, ctx.interest(&alice, &product.id));

            ctx.set_block_timestamp_in_ms(star_time + 6 * MS_IN_HOUR);
            assert_eq!(25_000_000_000_000_000, ctx.interest(&alice, &product.id));

            ctx.set_block_timestamp_in_ms(star_time + 12 * MS_IN_HOUR);
            assert_eq!(50_000_000_000_000_000, ctx.interest(&alice, &product.id));
        }

        let claimed_amount = ctx.claim_total(&alice);
        assert_eq!(50_000_000_000_000_000, claimed_amount);

        assert_eq!(0, ctx.interest(&alice, &product.id));

        {
            ctx.set_block_timestamp_in_ms(star_time + 14 * MS_IN_HOUR);
            assert_eq!(8_333_333_333_333_333, ctx.interest(&alice, &product.id));

            ctx.set_block_timestamp_in_ms(star_time + 18 * MS_IN_HOUR);
            assert_eq!(25_000_000_000_000_000, ctx.interest(&alice, &product.id));

            ctx.set_block_timestamp_in_ms(star_time + 24 * MS_IN_HOUR);
            assert_eq!(50_000_000_000_000_000, ctx.interest(&alice, &product.id));
        }
    }

    #[rstest]
    fn when_include_booster_then_return_compound_capped_apy(#[with(20_000, 5_000)] daily_score: DailyScore) {
        assert_eq!(UDecimal::new(25_000, 5), daily_score.to_capped_apy(30_000, true));
        assert_eq!(UDecimal::new(6_000, 5), daily_score.to_capped_apy(1_000, true));
    }

    #[rstest]
    fn when_exclude_booster_then_return_score_capped_apy(#[with(20_000, 5_000)] daily_score: DailyScore) {
        assert_eq!(UDecimal::new(20_000, 5), daily_score.to_capped_apy(30_000, false));
        assert_eq!(UDecimal::new(1_000, 5), daily_score.to_capped_apy(1_000, false));
    }

    #[rstest]
    fn when_compound_apy_exceeds_type_bounds_shouldnt_fail(#[with(65_000, 20_000)] daily_score: DailyScore) {
        assert_eq!(UDecimal::new(85_000, 5), daily_score.to_capped_apy(Score::MAX, true));
    }

    #[rstest]
    fn when_compound_apy_exceeds_max_value_it_gets_capped(#[with(65_000, 50_000)] daily_score: DailyScore) {
        assert_eq!(UDecimal::new(100_000, 5), daily_score.to_capped_apy(Score::MAX, true));
    }
}

impl Context {
    pub(crate) fn interest(&self, account_id: &AccountId, product_id: &ProductId) -> TokenAmount {
        self.contract()
            .get_total_interest(account_id.clone())
            .amount
            .detailed
            .get(product_id)
            .map_or(0, |value| value.0)
    }

    pub(crate) fn claim_total(&mut self, account_id: &AccountId) -> TokenAmount {
        self.switch_account(account_id);
        let PromiseOrValue::Value(claim_result) = self.contract().claim_total(None) else {
            panic!("Expected value");
        };

        claim_result.get_total().0
    }

    pub(crate) fn record_score(&mut self, account_id: &AccountId, time: UTC, score: Score) {
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

    pub(crate) fn score(&self, account_id: &AccountId) -> AccountScore {
        self.contract().get_account(account_id).score
    }
}
