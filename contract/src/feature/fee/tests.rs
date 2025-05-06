#![cfg(test)]

use near_sdk::{AccountId, PromiseOrValue};
use rstest::rstest;
use sweat_jar_model::api::FeeApi;
use crate::common::{env::test_env_ext, testing::{accounts::admin, Context}};

#[rstest]
fn withdraw_fee_success(admin: AccountId) {
    let mut context = Context::new(admin);
    let fee_amount = 1_000_000;
    context.contract().fee_amount = fee_amount;

    context.switch_account_to_manager();
    let withdrawn = match context.contract().withdraw_fee() {
        PromiseOrValue::Promise(_) => panic!("Expected value"),
        PromiseOrValue::Value(value) => value.0,
    };

    assert_eq!(withdrawn, fee_amount);
    assert_eq!(context.contract().get_fee_amount().0, 0);
}

#[rstest] 
fn withdraw_fee_ft_transfer_failure(admin: AccountId) {
    test_env_ext::set_test_future_success(false);
    
    let mut context = Context::new(admin);
    let fee_amount = 1_000_000;
    context.contract().fee_amount = fee_amount;

    context.switch_account_to_manager();
    let withdrawn = match context.contract().withdraw_fee() {
        PromiseOrValue::Promise(_) => panic!("Expected value"), 
        PromiseOrValue::Value(value) => value.0,
    };

    assert_eq!(withdrawn, 0);
    assert_eq!(context.contract().get_fee_amount().0, fee_amount);
}


#[rstest]
#[should_panic(expected = "Can be performed only by admin")]
fn withdraw_fee_not_admin(admin: AccountId, #[values("alice.near", "bob.near")] account: AccountId) {
    let mut context = Context::new(admin);
    let fee_amount = 1_000_000;
    context.contract().fee_amount = fee_amount;

    context.switch_account(&account);
    context.contract().withdraw_fee();
}
