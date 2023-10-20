#![cfg(test)]

use model::U32;
use near_sdk::{json_types::U128, test_utils::accounts, AccountId, PromiseOrValue};

use crate::{
    claim::api::ClaimApi,
    common::{test_data::set_test_future_success, tests::Context, udecimal::UDecimal, MS_IN_YEAR},
    jar::{api::JarApi, model::Jar},
    product::model::{Apy, Product, WithdrawalFee},
    withdraw::api::WithdrawApi,
};

fn prepare_jar(product: &Product) -> (AccountId, Jar, Context) {
    let alice = accounts(0);
    let admin = accounts(1);

    let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
    let context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&[jar.clone()]);

    (alice, jar, context)
}

#[test]
#[should_panic(expected = "Account 'owner' doesn't exist")]
fn withdraw_locked_jar_before_maturity_by_not_owner() {
    let (_, _, mut context) = prepare_jar(&generate_product());

    context.contract.withdraw(U32(0), None);
}

#[test]
#[should_panic(expected = "The jar is not mature yet")]
fn withdraw_locked_jar_before_maturity_by_owner() {
    let (alice, jar, mut context) = prepare_jar(&generate_product());

    context.switch_account(&alice);
    context.contract.withdraw(U32(jar.id), None);
}

#[test]
#[should_panic(expected = "Account 'owner' doesn't exist")]
fn withdraw_locked_jar_after_maturity_by_not_owner() {
    let product = generate_product();
    let (_, jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
    context.contract.withdraw(U32(jar.id), None);
}

#[test]
fn withdraw_locked_jar_after_maturity_by_owner() {
    let product = generate_product();
    let (alice, jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
    context.switch_account(&alice);
    context.contract.withdraw(U32(jar.id), None);
}

#[test]
#[should_panic(expected = "Account 'owner' doesn't exist")]
fn withdraw_flexible_jar_by_not_owner() {
    let product = generate_flexible_product();
    let (_, jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_days(1);
    context.contract.withdraw(U32(jar.id), None);
}

#[test]
fn withdraw_flexible_jar_by_owner_full() {
    let product = generate_flexible_product();
    let (alice, reference_jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_days(1);
    context.switch_account(&alice);

    context.contract.withdraw(U32(reference_jar.id), None);
    context.contract.claim_total();
    let jar = context.contract.get_jar(alice.clone(), U32(reference_jar.id));
    assert_eq!(0, jar.principal.0);
}

#[test]
fn withdraw_flexible_jar_by_owner_with_sufficient_balance() {
    let product = generate_flexible_product();
    let (alice, reference_jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_days(1);
    context.switch_account(&alice);

    context.contract.withdraw(U32(0), Some(U128(100_000)));
    let jar = context.contract.get_jar(alice.clone(), U32(reference_jar.id));
    assert_eq!(900_000, jar.principal.0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn withdraw_flexible_jar_by_owner_with_insufficient_balance() {
    let product = generate_flexible_product();
    let (alice, jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_days(1);
    context.switch_account(&alice);
    context.contract.withdraw(U32(jar.id), Some(U128(2_000_000)));
}

#[test]
fn dont_delete_jar_after_withdraw_with_interest_left() {
    let product = generate_product()
        .lockup_term(MS_IN_YEAR)
        .apy(Apy::Constant(UDecimal::new(2, 1)));

    let (alice, _, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
    context.switch_account(&alice);

    let jar = context.contract.get_jar_internal(&alice, 0);

    let PromiseOrValue::Value(withdrawn) = context.contract.withdraw(U32(jar.id), Some(U128(1_000_000))) else {
        panic!();
    };

    assert_eq!(withdrawn.withdrawn_amount, U128(1_000_000));
    assert_eq!(withdrawn.fee, U128(0));

    let jar = context.contract.get_jar_internal(&alice, 0);
    assert_eq!(jar.principal, 0);

    let Some(ref cache) = jar.cache else {
        panic!();
    };

    assert_eq!(cache.interest, 200_000);
}

#[test]
fn product_with_fixed_fee() {
    let fee = 10;
    let product = generate_product_with_fee(&WithdrawalFee::Fix(fee));
    let (alice, reference_jar, mut context) = prepare_jar(&product);

    let initial_principal = reference_jar.principal;

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
    context.switch_account(&alice);

    let withdraw_amount = 100_000;
    let PromiseOrValue::Value(withdraw) = context.contract.withdraw(U32(0), Some(U128(withdraw_amount))) else {
        panic!("Invalid promise type");
    };

    assert_eq!(withdraw.withdrawn_amount, U128(withdraw_amount - fee));
    assert_eq!(withdraw.fee, U128(fee));

    let jar = context.contract.get_jar(alice, U32(reference_jar.id));

    assert_eq!(jar.principal, U128(initial_principal - withdraw_amount));
}

#[test]
fn product_with_percent_fee() {
    let fee_value = UDecimal::new(5, 4);
    let fee = WithdrawalFee::Percent(fee_value.clone());
    let product = generate_product_with_fee(&fee);
    let (alice, reference_jar, mut context) = prepare_jar(&product);

    let initial_principal = reference_jar.principal;

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
    context.switch_account(&alice);

    let withdrawn_amount = 100_000;
    let PromiseOrValue::Value(withdraw) = context.contract.withdraw(U32(0), Some(U128(withdrawn_amount))) else {
        panic!("Invalid promise type");
    };

    let reference_fee = fee_value * initial_principal;
    assert_eq!(withdraw.withdrawn_amount, U128(withdrawn_amount - reference_fee));
    assert_eq!(withdraw.fee, U128(reference_fee));

    let jar = context.contract.get_jar(alice, U32(reference_jar.id));

    assert_eq!(jar.principal, U128(initial_principal - withdrawn_amount));
}

#[test]
fn test_failed_withdraw_promise() {
    set_test_future_success(false);

    let product = generate_product();
    let (alice, reference_jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
    context.switch_account(&alice);

    let jar_before_withdrawal = context.contract.get_jar(alice.clone(), U32(reference_jar.id));

    let PromiseOrValue::Value(withdrawn) = context.contract.withdraw(U32(0), Some(U128(100_000))) else {
        panic!()
    };

    assert_eq!(withdrawn.withdrawn_amount.0, 0);

    let jar_after_withdrawal = context.contract.get_jar(alice.clone(), U32(reference_jar.id));

    assert_eq!(jar_before_withdrawal, jar_after_withdrawal);
}

#[test]
fn test_failed_withdraw_internal() {
    let product = generate_product();
    let (alice, reference_jar, mut context) = prepare_jar(&product);
    let withdrawn_amount = 1_234;

    let jar_view = context.contract.get_jar(alice.clone(), U32(reference_jar.id));
    let jar = context
        .contract
        .account_jars
        .get(&alice)
        .unwrap()
        .iter()
        .next()
        .unwrap();

    let withdraw =
        context
            .contract
            .after_withdraw_internal(jar.account_id.clone(), jar.id, true, withdrawn_amount, None, false);

    assert_eq!(withdraw.withdrawn_amount, U128(0));
    assert_eq!(withdraw.fee, U128(0));

    assert_eq!(
        jar_view.principal.0 + withdrawn_amount,
        context.contract.get_jar(alice, U32(0)).principal.0
    );
}

#[test]
#[should_panic(expected = "Another operation on this Jar is in progress")]
fn withdraw_from_locked_jar() {
    let product = Product::generate("product")
        .apy(Apy::Constant(UDecimal::new(1, 0)))
        .lockup_term(MS_IN_YEAR);
    let mut jar = Jar::generate(0, &accounts(0), &product.id).principal(MS_IN_YEAR as u128);

    jar.lock();

    let alice = accounts(0);
    let admin = accounts(1);

    let mut context = Context::new(admin).with_products(&[product.clone()]).with_jars(&[jar]);

    context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
    context.switch_account(&alice);

    _ = context.contract.withdraw(U32(0), Some(U128(100_000)));
}

pub(crate) fn generate_product() -> Product {
    Product::generate("product").enabled(true)
}

pub(crate) fn generate_flexible_product() -> Product {
    Product::generate("flexible_product").enabled(true).flexible()
}

pub(crate) fn generate_product_with_fee(fee: &WithdrawalFee) -> Product {
    Product::generate("product_with_fee")
        .enabled(true)
        .with_withdrawal_fee(fee.clone())
}
