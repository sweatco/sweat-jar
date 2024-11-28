#![cfg(test)]

use std::panic::{catch_unwind, UnwindSafe};

use near_sdk::{AccountId, PromiseOrValue};
use sweat_jar_model::{TokenAmount, UDecimal, MS_IN_YEAR};

use crate::{
    common::Timestamp,
    jar::model::{Deposit, Jar},
    product::{
        helpers::MessageSigner,
        model::{Apy, DowngradableApy, FixedProductTerms, Product, Terms},
    },
};

/// Default product name. If product name wasn't specified it will have this name.
pub(crate) const DEFAULT_PRODUCT_NAME: &str = "product";
pub(crate) const DEFAULT_SCORE_PRODUCT_NAME: &str = "score_product";

pub fn admin() -> AccountId {
    "admin".parse().unwrap()
}

impl Jar {
    pub(crate) fn new() -> Self {
        Jar {
            deposits: vec![],
            cache: None,
            claimed_balance: 0,
            is_pending_withdraw: false,
            claim_remainder: 0,
        }
    }

    pub(crate) fn total_principal(&self) -> TokenAmount {
        self.deposits.iter().map(|deposit| deposit.principal).sum()
    }

    pub(crate) fn with_deposit(mut self, created_at: Timestamp, principal: TokenAmount) -> Self {
        self.deposits.push(Deposit::new(created_at, principal));
        self
    }

    pub(crate) fn with_deposits(mut self, deposits: Vec<(Timestamp, TokenAmount)>) -> Self {
        self.deposits.extend(
            deposits
                .into_iter()
                .map(|(created_at, deposit)| Deposit::new(created_at, deposit)),
        );
        self
    }

    pub(crate) fn with_pending_withdraw(mut self) -> Self {
        self.is_pending_withdraw = true;
        self
    }
}

pub fn generate_premium_product(id: &str, signer: &MessageSigner) -> Product {
    Product::new()
        .id(id)
        .public_key(signer.public_key())
        .cap(0, 100_000_000_000)
        .terms(Terms::Fixed(FixedProductTerms {
            apy: Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }),
            lockup_term: MS_IN_YEAR,
        }))
}

pub trait AfterCatchUnwind {
    fn after_catch_unwind(&self);
}

impl AfterCatchUnwind for () {
    fn after_catch_unwind(&self) {}
}

pub fn expect_panic(ctx: &impl AfterCatchUnwind, msg: &str, action: impl FnOnce() + UnwindSafe) {
    let res = catch_unwind(action);

    let panic_msg = res
        .err()
        .unwrap_or_else(|| panic!("Contract didn't panic when expected to.\nExpected message: {msg}"));

    if msg.is_empty() {
        ctx.after_catch_unwind();
        return;
    }

    let panic_msg = if let Some(msg) = panic_msg.downcast_ref::<&str>() {
        msg.to_string()
    } else if let Some(msg) = panic_msg.downcast_ref::<String>() {
        msg.clone()
    } else {
        panic!("Contract didn't panic with String or &str.\nExpected message: {msg}")
    };

    assert!(
        panic_msg.contains(msg),
        "Expected panic message to contain: {msg}.\nPanic message: {panic_msg}"
    );

    ctx.after_catch_unwind();
}

pub trait UnwrapPromise<T> {
    fn unwrap(self) -> T;
}

impl<T> UnwrapPromise<T> for PromiseOrValue<T> {
    fn unwrap(self) -> T {
        let PromiseOrValue::Value(t) = self else {
            panic!("Failed to unwrap PromiseOrValue")
        };
        t
    }
}

#[test]
#[should_panic(expected = "Contract didn't panic when expected to.\nExpected message: Something went wrong")]
fn test_expect_panic() {
    struct Ctx;
    impl AfterCatchUnwind for Ctx {
        fn after_catch_unwind(&self) {}
    }

    expect_panic(&Ctx, "Something went wrong", || {
        panic!("{}", "Something went wrong");
    });

    expect_panic(&Ctx, "Something went wrong", || {});
}
