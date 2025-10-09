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
    Timezone, MS_IN_DAY, MS_IN_HOUR, MS_IN_YEAR, UTC,
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
    context.contract().apply_booser(vec![alice.clone()], 5_000, 0.into());

    context.set_block_timestamp_in_ms(MS_IN_DAY);

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(50.to_otto(), interest.amount.total.0);

    context.set_block_timestamp_in_ms(2 * MS_IN_DAY);
    context
        .contract()
        .apply_booser(vec![alice.clone()], 5_000, MS_IN_DAY.into());

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(100.to_otto(), interest.amount.total.0);

    context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
    context
        .contract()
        .apply_booser(vec![alice.clone()], 10_000, (2 * MS_IN_DAY).into());

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

    // Day 1: Apply booster (5,000) for yestarday
    context.set_block_timestamp_in_ms(MS_IN_DAY);
    context.contract().apply_booser(vec![alice.clone()], 5_000, 0.into());

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(150.to_otto(), interest.amount.total.0);

    // Day 2: Record more regular score (5,000) for yesterday
    context.set_block_timestamp_in_ms(2 * MS_IN_DAY + MS_IN_HOUR);
    context
        .contract()
        .record_score(vec![(alice.clone(), vec![(5_000, (MS_IN_DAY + MS_IN_HOUR).into())])]);

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(200.to_otto(), interest.amount.total.0);

    // Day 3: Apply another booster (10,000) for yesterday
    context.set_block_timestamp_in_ms(3 * MS_IN_DAY);
    context
        .contract()
        .apply_booser(vec![alice.clone()], 10_000, (2 * MS_IN_DAY).into());

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
    context.contract().apply_booser(vec![alice.clone()], 8_000, 0.into());

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
    let pending_boosters_after = account_after_claim.score.get_pending_boosters();
    assert_eq!(pending_boosters_after, 0); // Booster should not be available
}
