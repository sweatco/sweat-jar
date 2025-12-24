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
    Timezone, MS_IN_DAY, MS_IN_HOUR, MS_IN_YEAR,
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

    let start_time = MS_IN_DAY * 2_000;

    context
        .contract()
        .accounts
        .set(alice.clone(), AccountVersioned::new(Account::default()).into());
    context.contract().set_timezone(alice.clone(), 0.into());
    context
        .contract()
        .get_account_mut(&alice)
        .deposit(&product.id, 365_000.to_otto(), start_time.into());

    // Day 0: Record regular score that exceeds the cap (25,000 > 10,000 cap)
    {
        context.set_block_timestamp_in_ms(start_time);
        context
            .contract()
            .record_score(vec![(alice.clone(), vec![(25_000, (start_time - MS_IN_HOUR).into())])]);

        assert_eq!(25_000, context.contract().get_score(alice.clone()).unwrap().0);
    }

    // Day 1: Apply booster (5,000) for yesterday
    {
        let now = start_time + MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);

        // Check interest from Day -1 score
        assert_eq!(100.to_otto(), context.interest(&alice, &product.id));

        context
            .contract()
            .apply_booster(vec![alice.clone()], 5_000, (now - MS_IN_HOUR).into());
    }

    // Day 2: Record more regular score (5,000) for yesterday
    {
        let now = start_time + 2 * MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);

        dbg!(context.contract().get_account(&alice));

        // Check interest from Day 0 score
        assert_eq!(150.to_otto(), context.interest(&alice, &product.id));
    }

    // Day 3: Apply another booster (10,000) for yesterday
    {
        let now = start_time + 3 * MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);

        context
            .contract()
            .apply_booster(vec![alice.clone()], 10_000, (now - MS_IN_HOUR).into());

        context
            .contract()
            .record_score(vec![(alice.clone(), vec![(10_000, (now - MS_IN_HOUR).into())])]);
    }

    // Day 4
    {
        let now = start_time + 4 * MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);

        assert_eq!(350.to_otto(), context.interest(&alice, &product.id));
    }
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
}
