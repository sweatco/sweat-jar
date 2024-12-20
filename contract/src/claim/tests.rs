#![cfg(test)]

use near_sdk::{json_types::U128, test_utils::test_env::alice, PromiseOrValue};
use sweat_jar_model::{
    api::{ClaimApi, WithdrawApi},
    claimed_amount_view::ClaimedAmountView,
    product::{Apy, FixedProductTerms, Product, Terms},
    UDecimal, MS_IN_DAY, MS_IN_MINUTE, MS_IN_YEAR,
};

use crate::{
    common::{test_data::set_test_future_success, tests::Context},
    jar::model::Jar,
    product::model::v1::InterestCalculator,
    test_utils::{admin, UnwrapPromise},
};

#[test]
fn claim_total_when_nothing_to_claim() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.switch_account(alice);
    let value = context.contract().claim_total(None).unwrap();

    assert_eq!(0, value.get_total().0);
}

#[test]
fn claim_total_detailed_when_having_tokens() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let jar = Jar::new().with_deposit(0, 100_000_000).with_deposit(1, 200_000_000);
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

#[test]
fn claim_pending_withdraw_jar() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let jar = Jar::new()
        .with_deposit(0, 100_000_000)
        .with_deposit(1, 200_000_000)
        .with_pending_withdraw();
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

#[test]
fn dont_delete_jar_on_all_interest_claim() {
    let alice = alice();
    let admin = admin();

    let product = Product::default().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR.into(),
        apy: Apy::Constant(UDecimal::new(2, 1)),
    }));
    let jar = Jar::new().with_deposit(0, 800_000).with_deposit(MS_IN_DAY, 200_000);
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

#[test]
#[should_panic(expected = "Jar for product product is not found")]
fn claim_all_withdraw_all_and_delete_jar() {
    let alice = alice();
    let admin = admin();

    let lockup_term = 3 * MS_IN_YEAR;
    let product = Product::default().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: lockup_term.into(),
        apy: Apy::Constant(UDecimal::new(2, 1)),
    }));
    let jar = Jar::new().with_deposit(0, 500_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(lockup_term + MS_IN_DAY);

    context.switch_account(&alice);
    let claimed = context.contract().claim_total(None).unwrap();

    assert_eq!(300_000, claimed.get_total().0);

    let jar = context.contract().get_account(&alice).get_jar(&product.id).clone();
    let Some(ref cache) = jar.cache else { panic!() };

    assert_eq!(cache.interest, 0);
    assert_eq!(jar.total_principal(), 500_000);

    let withdrawn = context.contract().withdraw(product.id.clone()).unwrap();

    assert_eq!(withdrawn.withdrawn_amount, U128(500_000));
    assert_eq!(withdrawn.fee, U128(0));

    let _jar = context.contract().get_account(&alice).get_jar(&product.id);
}

#[test]
#[should_panic(expected = "Jar for product testing_product is not found")]
fn withdraw_all_claim_all_and_delete_jar() {
    let alice = alice();
    let admin = admin();

    let lockup_term = 2 * MS_IN_YEAR;
    let product = Product::default()
        .with_id("testing_product")
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: lockup_term.into(),
            apy: Apy::Constant(UDecimal::new(1, 1)),
        }));
    let jar = Jar::new().with_deposit(0, 1_500_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(lockup_term + MS_IN_MINUTE);

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

#[test]
fn failed_future_claim() {
    set_test_future_success(false);

    let alice = alice();
    let admin = admin();

    let lockup_term = MS_IN_YEAR;
    let product = Product::default()
        .with_id("broken_product")
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: lockup_term.into(),
            apy: Apy::Constant(UDecimal::new(2, 1)),
        }));
    let jar = Jar::new().with_deposit(0, 700_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(lockup_term + MS_IN_DAY);

    context.switch_account(&alice);

    let jar_before_claim = context.contract().get_account(&alice).get_jar(&product.id).clone();

    let claimed = context.contract().claim_total(None).unwrap();

    assert_eq!(claimed.get_total().0, 0);

    let jar_after_claim = context.contract().get_account(&alice).get_jar(&product.id).clone();

    assert_eq!(jar_before_claim, jar_after_claim);
}
