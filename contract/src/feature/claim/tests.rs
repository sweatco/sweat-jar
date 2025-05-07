#![cfg(test)]

use fake::Fake;
use near_sdk::{json_types::U128, AccountId, PromiseOrValue};
use rstest::rstest;
use sweat_jar_model::{
    api::{AccountApi, ClaimApi, WithdrawApi},
    data::{claim::ClaimedAmountView, jar::Jar, product::Product},
    interest::InterestCalculator,
    TokenAmount, MS_IN_DAY, MS_IN_MINUTE, MS_IN_YEAR,
};

use crate::{
    common::{
        env::test_env_ext,
        event::EventKind,
        testing::{accounts::*, Context, UnwrapPromise},
    },
    feature::{
        account::model::test_utils::{jar, JarBuilder},
        product::model::test_utils::*,
    },
};

#[rstest]
fn claim_total_when_nothing_to_claim(
    alice: AccountId,
    admin: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.switch_account(alice);
    let value = context.contract().claim_total(None).unwrap();

    assert_eq!(0, value.get_total().0);
}

#[rstest]
fn claim_total_detailed_when_having_tokens(
    alice: AccountId,
    admin: AccountId,
    #[from(product_1_year_apy_20_percent)] product: Product,
    #[with(vec![(0, 100_000_000), (1, 200_000_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    let test_duration = MS_IN_YEAR + MS_IN_DAY;

    let expected_interest = product
        .terms
        .get_interest(context.contract().get_account(&alice), &jar, test_duration);
    assert_eq!(60_000_000, expected_interest.0);

    context.set_block_timestamp_in_ms(test_duration);

    context.switch_account(&alice);
    let result = context.contract().claim_total(Some(true));

    let PromiseOrValue::Value(ClaimedAmountView::Detailed(value)) = result else {
        panic!();
    };

    assert_eq!(expected_interest.0, value.total.0);
    assert_eq!(1, value.detailed.len());
    assert_eq!(expected_interest.0, value.detailed.get(&product.id).unwrap().0);
}

#[rstest]
fn claim_pending_withdraw_jar(
    alice: AccountId,
    admin: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000), (1, 200_000_000)])] jar: Jar,
) {
    let jar = jar.with_pending_withdraw();
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    let test_duration = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(test_duration);

    context.switch_account(&alice);
    let result = context.contract().claim_total(Some(true));

    let PromiseOrValue::Value(ClaimedAmountView::Detailed(value)) = result else {
        panic!();
    };

    assert_eq!(0, value.total.0);
    assert_eq!(0, value.detailed.len());
}

#[rstest]
fn dont_delete_jar_on_all_interest_claim(
    alice: AccountId,
    admin: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 800_000), (MS_IN_DAY, 200_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(MS_IN_YEAR + 2 * MS_IN_DAY);

    context.switch_account(&alice);
    context.contract().claim_total(None);

    let jar = context.contract().get_account(&alice).get_jar(&product.id).clone();
    let Some(ref cache) = jar.cache else { panic!() };

    assert_eq!(cache.interest, 0);
    assert_eq!(jar.total_principal(), 1_000_000);
}

#[rstest]
#[should_panic(expected = "Jar for product product_3_years_20_percent is not found")]
fn claim_all_withdraw_all_and_delete_jar(
    alice: AccountId,
    admin: AccountId,
    #[from(product_3_years_20_percent)] product: Product,
    #[with(vec![(0, 500_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + MS_IN_DAY);

    context.switch_account(&alice);
    let claimed = context.contract().claim_total(None).unwrap();

    assert_eq!(300_000, claimed.get_total().0);
    let events = context.get_events();
    let EventKind::Claim(_, claim_data) = events.last().unwrap() else {
        panic!("Expected Claim event");
    };
    assert_eq!(1, claim_data.items.len());

    let jar = context.contract().get_account(&alice).get_jar(&product.id).clone();
    let Some(ref cache) = jar.cache else { panic!() };

    assert_eq!(cache.interest, 0);
    assert_eq!(jar.total_principal(), 500_000);

    let withdrawn = context.contract().withdraw(product.id.clone()).unwrap();

    assert_eq!(withdrawn.withdrawn_amount, U128(500_000));
    assert_eq!(withdrawn.fee, U128(0));

    let _jar = context.contract().get_account(&alice).get_jar(&product.id);
}

#[rstest]
#[should_panic(expected = "Jar for product product_2_years_10_percent is not found")]
fn withdraw_all_claim_all_and_delete_jar(
    alice: AccountId,
    admin: AccountId,
    #[from(product_2_years_10_percent)] product: Product,
    #[with(vec![(0, 1_500_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + MS_IN_MINUTE);

    context.switch_account(&alice);

    let withdrawn = context.contract().withdraw(product.id.clone()).unwrap();

    assert_eq!(withdrawn.withdrawn_amount, U128(1_500_000));
    assert_eq!(withdrawn.fee, U128(0));

    let jar = context.contract().get_account(&alice).get_jar(&product.id).clone();
    assert_eq!(jar.total_principal(), 0);

    let claimed = context.contract().claim_total(None).unwrap();
    assert_eq!(claimed.get_total(), U128(300_000));

    let _jar = context.contract().get_account(&alice).get_jar(&product.id).clone();
}

#[rstest]
fn failed_future_claim(
    alice: AccountId,
    admin: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 700_000)])] jar: Jar,
) {
    test_env_ext::set_test_future_success(false);

    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + MS_IN_DAY);

    context.switch_account(&alice);

    let jar_before_claim = context.contract().get_account(&alice).get_jar(&product.id).clone();

    let claimed = context.contract().claim_total(None).unwrap();

    assert_eq!(claimed.get_total().0, 0);

    let jar_after_claim = context.contract().get_account(&alice).get_jar(&product.id).clone();

    assert_eq!(jar_before_claim, jar_after_claim);
}

#[rstest]
fn claim_often_vs_claim_once(#[from(product_1_year_12_percent)] product: Product) {
    fn test(mut product: Product, principal: TokenAmount, days: u64, n: usize) {
        test_env_ext::set_test_log_events(false);

        let alice: AccountId = format!("alice_{principal}_{days}_{n}").try_into().unwrap();
        let bob: AccountId = format!("bob_{principal}_{days}_{n}").try_into().unwrap();
        let admin: AccountId = format!("admin_{principal}_{days}_{n}").try_into().unwrap();

        product.id = format!("product_{principal}_{days}_{n}");

        let alice_jar = jar(vec![(0, principal)]);
        let bob_jar = jar(vec![(0, principal)]);

        let mut context = Context::new(admin)
            .with_products(&[product.clone()])
            .with_jars(&alice, &[(product.id.clone(), alice_jar)])
            .with_jars(&bob, &[(product.id.clone(), bob_jar)]);

        let mut bobs_claimed = 0;

        context.switch_account(&bob);

        for day in 0..days {
            context.set_block_timestamp_in_days(day);
            let claimed = context.contract().claim_total(None).unwrap();
            bobs_claimed += claimed.get_total().0;
        }

        let alice_interest = context.contract().get_total_interest(alice.clone()).amount.total.0;

        assert_eq!(alice_interest, bobs_claimed);
    }

    test(product.clone(), 10_000_000_000_000_000_000_000_000_000, 365, 0);

    for n in 1..10 {
        test(
            product.clone(),
            (1..10_000_000_000_000_000_000_000_000_000).fake(),
            (1..365).fake(),
            n,
        );
    }
}
