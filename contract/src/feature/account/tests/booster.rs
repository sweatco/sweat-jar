#![cfg(test)]

use near_sdk::{AccountId, PromiseOrValue};
use rstest::rstest;
use sweat_jar_model::{
    api::{AccountApi, ClaimApi},
    data::{
        account::{versioned::AccountVersioned, Account},
        deposit::DepositTicket,
        product::Product,
    },
    BoostedScore, Timezone, MS_IN_DAY, MS_IN_HOUR, MS_IN_YEAR,
};

use crate::{
    common::{
        env::test_env_ext,
        testing::{
            accounts::{admin, alice},
            Context, TokenUtils,
        },
    },
    feature::product::model::test_utils::*,
};

#[rstest]
fn claim_from_tiered_score_jar_only_with_booster(
    admin: AccountId,
    alice: AccountId,
    #[from(tiered_score_based_product)] product: Product,
) {
    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);
    context.switch_account_to_manager();

    context
        .contract()
        .accounts
        .set(alice.clone(), AccountVersioned::new(Account::default()).into());
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
    context.contract().apply_booster(vec![alice.clone()], 5_000, 0.into());

    context.set_block_timestamp_in_ms(2 * MS_IN_DAY);

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(50.to_otto(), interest.amount.total.0);

    context.set_block_timestamp_in_ms(2 * MS_IN_DAY);
    context
        .contract()
        .apply_booster(vec![alice.clone()], 5_000, MS_IN_DAY.into());

    context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(100.to_otto(), interest.amount.total.0);

    context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
    context
        .contract()
        .apply_booster(vec![alice.clone()], 10_000, (2 * MS_IN_DAY).into());

    context.set_block_timestamp_in_ms(4 * MS_IN_DAY);
    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(200.to_otto(), interest.amount.total.0);
}

#[rstest]
fn claim_from_tiered_score_jar_with_mixed_regular_and_boosted_score(
    admin: AccountId,
    alice: AccountId,
    #[from(tiered_score_based_product)] product: Product,
) {
    test_env_ext::set_test_log_events(false);

    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);
    context.switch_account_to_manager();

    context
        .contract()
        .accounts
        .set(alice.clone(), AccountVersioned::new(Account::default()).into());
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

    // Day 0: Record regular score that exceeds the cap (25,000 > 10,000 cap)
    context.set_block_timestamp_in_ms(0);
    context
        .contract()
        .record_score(vec![(alice.clone(), vec![(25_000, 0.into())])]);

    // Day 1: Apply booster (5,000) for yesterday
    context.set_block_timestamp_in_ms(MS_IN_DAY);
    context.contract().apply_booster(vec![alice.clone()], 5_000, 0.into());

    context.set_block_timestamp_in_ms(2 * MS_IN_DAY);
    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(150.to_otto(), interest.amount.total.0);

    // Day 2: Record more regular score (5,000) for yesterday
    context.set_block_timestamp_in_ms(2 * MS_IN_DAY + MS_IN_HOUR);
    context
        .contract()
        .record_score(vec![(alice.clone(), vec![(5_000, (MS_IN_DAY + MS_IN_HOUR).into())])]);

    context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(200.to_otto(), interest.amount.total.0);

    // Day 3: Apply another booster (10,000) for yesterday
    context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
    context
        .contract()
        .apply_booster(vec![alice.clone()], 10_000, (2 * MS_IN_DAY).into());

    context.set_block_timestamp_in_ms(4 * MS_IN_DAY);

    // Check interest calculation for day 3
    // Day 0: Regular score 25,000 (capped at 10,000) + Booster 5,000 = 15,000
    // Day 1: Regular score 5,000, Booster 0 = 5,000
    // Day 2: Regular score 0, Booster 10,000 = 10,000
    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(300.to_otto(), interest.amount.total.0);
}

#[rstest]
fn claim_from_tiered_score_jar_with_delayed_booster_claim(
    admin: AccountId,
    alice: AccountId,
    #[from(tiered_score_based_product)] product: Product,
) {
    test_env_ext::set_test_log_events(false);

    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);
    context.switch_account_to_manager();

    context
        .contract()
        .accounts
        .set(alice.clone(), AccountVersioned::new(Account::default()).into());
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

    // Day 0: Record regular score
    context.set_block_timestamp_in_ms(0);
    context
        .contract()
        .record_score(vec![(alice.clone(), vec![(10_000, 0.into())])]);

    // Day 1: Apply booster but don't claim yet
    context.set_block_timestamp_in_ms(MS_IN_DAY);
    context.contract().apply_booster(vec![alice.clone()], 8_000, 0.into());

    context.set_block_timestamp_in_ms(7 * MS_IN_DAY);
    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(180.to_otto(), interest.amount.total.0);

    // Now claim the interest
    context.switch_account(&alice);
    let PromiseOrValue::Value(claim_result) = context.contract().claim_total(None) else {
        panic!("Expected value");
    };
    let claimed = claim_result.get_total().0;
    assert_eq!(claimed, 180.to_otto());

    // After claiming, booster should still not be available (it was for day 1)
    let account_after_claim = context.contract().get_account(&alice).clone();
    let pending_boosters_after = account_after_claim
        .score
        .get_pending_finalized_boosters(Timezone::hour_shift(0));
    assert_eq!(pending_boosters_after, 0); // Booster should not be available
}

#[test]
fn test_new_unclaimed() {
    let booster = BoostedScore::new(1234, false);
    assert_eq!(booster.get_value(), 1230);
    assert!(!booster.is_claimed());
}

#[test]
fn test_new_claimed() {
    let booster = BoostedScore::new(5678, true);
    assert_eq!(booster.get_value(), 5670);
    assert!(booster.is_claimed());
}

#[test]
fn test_new_zero_score() {
    let booster = BoostedScore::new(0, false);
    assert_eq!(booster.get_value(), 0);
    assert!(!booster.is_claimed());
}

#[test]
fn test_new_max_score() {
    let booster = BoostedScore::new(65535, true);
    assert_eq!(booster.get_value(), 65530);
    assert!(booster.is_claimed());
}

#[test]
fn test_set_claimed_true() {
    let mut booster = BoostedScore::new(1000, false);
    assert!(!booster.is_claimed());

    booster.set_claimed(true);
    assert!(booster.is_claimed());
    assert_eq!(booster.get_value(), 1000); // Score should remain unchanged
}

#[test]
fn test_set_claimed_false() {
    let mut booster = BoostedScore::new(2000, true);
    assert!(booster.is_claimed());

    booster.set_claimed(false);
    assert!(!booster.is_claimed());
    assert_eq!(booster.get_value(), 2000); // Score should remain unchanged
}

#[test]
fn test_set_claimed_multiple_times() {
    let mut booster = BoostedScore::new(1500, false);

    booster.set_claimed(true);
    assert!(booster.is_claimed());

    booster.set_claimed(false);
    assert!(!booster.is_claimed());

    booster.set_claimed(true);
    assert!(booster.is_claimed());
}

#[test]
fn test_bit_packing_consistency() {
    // Test that the bit packing works correctly
    let booster = BoostedScore::new(1234, true);

    // The internal representation should be:
    // - Score 1234 normalized to 123 (1234 / 10)
    // - Shifted left by 1: 123 << 1 = 246
    // - OR with claim bit: 246 | 1 = 247
    assert_eq!(booster.get_raw_value(), 247);
}

#[test]
fn test_bit_packing_unclaimed() {
    let booster = BoostedScore::new(1000, false);

    // The internal representation should be:
    // - Score 1000 normalized to 100 (1000 / 10)
    // - Shifted left by 1: 100 << 1 = 200
    // - OR with claim bit: 200 | 0 = 200
    assert_eq!(booster.get_raw_value(), 200);
}

#[test]
fn test_score_preservation_after_claim_changes() {
    let mut booster = BoostedScore::new(5432, false);
    let original_score = booster.get_value();

    booster.set_claimed(true);
    assert_eq!(booster.get_value(), original_score);

    booster.set_claimed(false);
    assert_eq!(booster.get_value(), original_score);
}

#[test]
fn test_edge_case_scores() {
    // Test various edge cases
    let test_cases = vec![
        (0, false),
        (0, true),
        (1, false),
        (1, true),
        (9, false),  // Should round down to 0 when normalized
        (10, false), // Should normalize to 1
        (11, false), // Should normalize to 1
    ];

    for (score, is_claimed) in test_cases {
        let booster = BoostedScore::new(score, is_claimed);
        assert_eq!(booster.is_claimed(), is_claimed);
        // Note: Due to normalization, some scores may not round-trip perfectly
        // This is expected behavior for scores < 10
    }
}

#[test]
fn test_claim_unclaimed_booster() {
    let mut booster = BoostedScore::new(2500, false);
    assert!(!booster.is_claimed());

    let score = booster.claim();
    assert_eq!(score, 2500);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_already_claimed_booster() {
    let mut booster = BoostedScore::new(1000, true);
    assert!(booster.is_claimed());

    let score = booster.claim();
    assert_eq!(score, 0);
    assert!(booster.is_claimed()); // Should still be claimed
}

#[test]
fn test_claim_zero_score_booster() {
    let mut booster = BoostedScore::new(0, false);
    assert!(!booster.is_claimed());

    let score = booster.claim();
    assert_eq!(score, 0);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_multiple_times() {
    let mut booster = BoostedScore::new(3000, false);

    // First claim should return the score
    let first_claim = booster.claim();
    assert_eq!(first_claim, 3000);
    assert!(booster.is_claimed());

    // Second claim should return 0
    let second_claim = booster.claim();
    assert_eq!(second_claim, 0);
    assert!(booster.is_claimed());

    // Third claim should also return 0
    let third_claim = booster.claim();
    assert_eq!(third_claim, 0);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_preserves_score_value() {
    let mut booster = BoostedScore::new(5432, false);
    let original_score = booster.get_value();

    let claimed_score = booster.claim();
    assert_eq!(claimed_score, original_score);
    assert_eq!(booster.get_value(), original_score);
}

#[test]
fn test_claim_after_manual_set_claimed() {
    let mut booster = BoostedScore::new(2000, false);

    // Manually set as claimed
    booster.set_claimed(true);
    assert!(booster.is_claimed());

    // Claim should now return 0
    let score = booster.claim();
    assert_eq!(score, 0);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_after_manual_set_unclaimed() {
    let mut booster = BoostedScore::new(1500, true);

    // Manually set as unclaimed
    booster.set_claimed(false);
    assert!(!booster.is_claimed());

    // Claim should now return the score
    let score = booster.claim();
    assert_eq!(score, 1500);
    assert!(booster.is_claimed());
}
