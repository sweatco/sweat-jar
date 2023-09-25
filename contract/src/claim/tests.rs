#![cfg(test)]

use near_sdk::{json_types::U128, test_utils::accounts, PromiseOrValue};

use crate::{
    claim::api::ClaimApi,
    common::{tests::Context, u32::U32, udecimal::UDecimal, MS_IN_YEAR},
    jar::{api::JarApi, model::Jar},
    product::model::{Apy, Product},
    withdraw::api::WithdrawApi,
};

#[test]
fn claim_total_when_nothing_to_claim() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product();
    let jar = Jar::generate(0, &alice, &product.id).principal(100_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar]);

    context.switch_account(&alice);
    let result = context.contract.claim_total();

    let PromiseOrValue::Value(value) = result else {
        panic!();
    };

    assert_eq!(0, value.0);
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
    let PromiseOrValue::Value(claimed) = context.contract.claim_jars(vec![U32(jar.id)], Some(U128(100))) else {
        panic!()
    };

    dbg!(&claimed);

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
    context.contract.claim_jars(vec![U32(jar.id)], Some(U128(200_000)));

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
    context.contract.claim_jars(vec![U32(jar_id)], Some(U128(200_000)));

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

    let PromiseOrValue::Value(claimed) = context.contract.claim_jars(vec![U32(jar_id)], Some(U128(200_000))) else {
        panic!();
    };

    assert_eq!(claimed, U128(200_000));

    let _jar = context.contract.get_jar_internal(&alice, jar_id);
}

fn generate_product() -> Product {
    Product::generate("product")
        .enabled(true)
        .lockup_term(MS_IN_YEAR)
        .apy(Apy::Constant(UDecimal::new(12, 2)))
}
