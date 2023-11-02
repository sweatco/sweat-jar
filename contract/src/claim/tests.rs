#![cfg(test)]

use model::{
    api::{ClaimApi, JarApi, WithdrawApi},
    MS_IN_YEAR, U32,
};
use near_sdk::{json_types::U128, test_utils::accounts, PromiseOrValue};

use crate::{
    common::{test_data::set_test_future_success, tests::Context, udecimal::UDecimal},
    jar::model::Jar,
    product::model::{Apy, Product},
};

#[test]
fn claim_total_when_nothing_to_claim() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product();
    let jar = Jar::generate(0, &alice, &product.id).principal(100_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar]);

    context.switch_account(&alice);
    let result = context.contract.claim_total(None);

    let PromiseOrValue::Value(value) = result else {
        panic!();
    };

    assert_eq!(0, value.get_total().0);
}

#[test]
fn claim_total_detailed_when_having_tokens() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product();
    let jar_0 = Jar::generate(0, &alice, &product.id).principal(100_000_000);
    let jar_1 = Jar::generate(1, &alice, &product.id).principal(200_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&[jar_0.clone(), jar_1.clone()]);

    let product_term = product.get_lockup_term().unwrap();
    let test_duration = product_term + 100;

    let jar_0_expected_interest = jar_0.get_interest(&product, test_duration);
    let jar_1_expected_interest = jar_1.get_interest(&product, test_duration);

    context.set_block_timestamp_in_ms(test_duration);

    context.switch_account(&alice);
    let result = context.contract.claim_total(Some(true));

    let PromiseOrValue::Value(ClaimedAmountView::Detailed(value)) = result else {
        panic!();
    };

    assert_eq!(jar_0_expected_interest + jar_1_expected_interest, value.total.0);

    assert_eq!(jar_0_expected_interest, value.detailed.get(&U32(jar_0.id)).unwrap().0);
    assert_eq!(jar_1_expected_interest, value.detailed.get(&U32(jar_1.id)).unwrap().0);
}

#[test]
fn claim_partially_detailed_when_having_tokens() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product();
    let jar_0 = Jar::generate(0, &alice, &product.id).principal(100_000_000);
    let jar_1 = Jar::generate(1, &alice, &product.id).principal(200_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&[jar_0.clone(), jar_1.clone()]);

    let product_term = product.get_lockup_term().unwrap();
    let test_duration = product_term + 100;

    let jar_0_expected_interest = jar_0.get_interest(&product, test_duration);
    let jar_1_expected_interest = jar_1.get_interest(&product, test_duration) - 1;

    context.set_block_timestamp_in_ms(test_duration);

    context.switch_account(&alice);
    let result = context.contract.claim_jars(
        vec![U32(jar_0.id), U32(jar_1.id)],
        Some(U128(jar_0_expected_interest + jar_1_expected_interest)),
        Some(true),
    );

    let PromiseOrValue::Value(ClaimedAmountView::Detailed(value)) = result else {
        panic!();
    };

    assert_eq!(jar_0_expected_interest + jar_1_expected_interest, value.total.0);

    assert_eq!(jar_0_expected_interest, value.detailed.get(&U32(jar_0.id)).unwrap().0);
    assert_eq!(jar_1_expected_interest, value.detailed.get(&U32(jar_1.id)).unwrap().0);
}

#[test]
fn claim_partially_detailed_when_having_tokens_and_request_sum_of_single_deposit() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product();
    let jar_0 = Jar::generate(0, &alice, &product.id).principal(100_000_000);
    let jar_1 = Jar::generate(1, &alice, &product.id).principal(200_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&[jar_0.clone(), jar_1.clone()]);

    let product_term = product.get_lockup_term().unwrap();
    let test_duration = product_term + 100;

    let jar_0_expected_interest = jar_0.get_interest(&product, test_duration);

    context.set_block_timestamp_in_ms(test_duration);

    context.switch_account(&alice);
    let result = context.contract.claim_jars(
        vec![U32(jar_0.id), U32(jar_1.id)],
        Some(U128(jar_0_expected_interest)),
        Some(true),
    );

    let PromiseOrValue::Value(ClaimedAmountView::Detailed(value)) = result else {
        panic!();
    };

    assert_eq!(jar_0_expected_interest, value.total.0);

    assert_eq!(jar_0_expected_interest, value.detailed.get(&U32(jar_0.id)).unwrap().0);
    assert!(!value.detailed.contains_key(&U32(jar_1.id)));
}

#[test]
fn claim_partially_when_having_tokens_to_claim() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product();
    let jar = Jar::generate(0, &alice, &product.id).principal(100_000_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_days(365);

    context.switch_account(&alice);
    let PromiseOrValue::Value(claimed) = context.contract.claim_jars(vec![U32(jar.id)], Some(U128(100)), None) else {
        panic!()
    };

    assert_eq!(claimed.get_total().0, 100);

    let jar = context.contract.get_jar(alice, U32(jar.id));
    assert_eq!(100, jar.claimed_balance.0);
}

#[test]
fn dont_delete_jar_on_all_interest_claim() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product().apy(Apy::Constant(UDecimal::new(2, 1)));
    let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_days(365);

    context.switch_account(&alice);
    context
        .contract
        .claim_jars(vec![U32(jar.id)], Some(U128(200_000)), None);

    let jar = context.contract.get_jar_internal(&alice, jar.id);
    assert_eq!(200_000, jar.claimed_balance);

    let Some(ref cache) = jar.cache else { panic!() };

    assert_eq!(cache.interest, 0);
    assert_eq!(jar.principal, 1_000_000);
}

#[test]
#[should_panic(expected = "Jar with id: 0 doesn't exist")]
fn claim_all_withdraw_all_and_delete_jar() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product().apy(Apy::Constant(UDecimal::new(2, 1)));
    let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);

    let jar_id = jar.id;

    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);

    context.switch_account(&alice);
    let PromiseOrValue::Value(claimed) = context
        .contract
        .claim_jars(vec![U32(jar_id)], Some(U128(200_000)), None)
    else {
        panic!()
    };

    assert_eq!(200_000, claimed.get_total().0);

    let jar = context.contract.get_jar_internal(&alice, jar_id);
    assert_eq!(200_000, jar.claimed_balance);

    let Some(ref cache) = jar.cache else { panic!() };

    assert_eq!(cache.interest, 0);
    assert_eq!(jar.principal, 1_000_000);

    let PromiseOrValue::Value(withdrawn) = context.contract.withdraw(U32(jar_id), None) else {
        panic!()
    };

    assert_eq!(withdrawn.withdrawn_amount, U128(1_000_000));
    assert_eq!(withdrawn.fee, U128(0));

    let _jar = context.contract.get_jar_internal(&alice, jar_id);
}

#[test]
#[should_panic(expected = "Jar with id: 0 doesn't exist")]
fn withdraw_all_claim_all_and_delete_jar() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product().apy(Apy::Constant(UDecimal::new(2, 1)));
    let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);

    let jar_id = jar.id;

    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);

    context.switch_account(&alice);

    let PromiseOrValue::Value(withdrawn) = context.contract.withdraw(U32(jar_id), None) else {
        panic!()
    };

    assert_eq!(withdrawn.withdrawn_amount, U128(1_000_000));
    assert_eq!(withdrawn.fee, U128(0));

    let jar = context.contract.get_jar_internal(&alice, jar_id);

    assert_eq!(jar.principal, 0);

    let PromiseOrValue::Value(claimed) = context
        .contract
        .claim_jars(vec![U32(jar_id)], Some(U128(200_000)), None)
    else {
        panic!();
    };

    assert_eq!(claimed.get_total(), U128(200_000));

    let _jar = context.contract.get_jar_internal(&alice, jar_id);
}

#[test]
fn failed_future_claim() {
    set_test_future_success(false);

    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product().apy(Apy::Constant(UDecimal::new(2, 1)));
    let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_days(365);

    context.switch_account(&alice);

    let jar_before_claim = context.contract.get_jar_internal(&alice, jar.id).clone();

    let PromiseOrValue::Value(claimed) = context
        .contract
        .claim_jars(vec![U32(jar.id)], Some(U128(200_000)), None)
    else {
        panic!()
    };

    assert_eq!(claimed.get_total().0, 0);

    let jar_after_claim = context.contract.get_jar_internal(&alice, jar.id);

    assert_eq!(&jar_before_claim, jar_after_claim);
}

fn generate_product() -> Product {
    Product::generate("product")
        .enabled(true)
        .lockup_term(MS_IN_YEAR)
        .apy(Apy::Constant(UDecimal::new(12, 2)))
}
