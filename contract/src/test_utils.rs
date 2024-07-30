#![cfg(test)]

use std::panic::{catch_unwind, UnwindSafe};

use near_sdk::{AccountId, PromiseOrValue};
use sweat_jar_model::TokenAmount;

use crate::{
    common::{udecimal::UDecimal, Timestamp},
    jar::model::{Jar, JarV1},
    product::{
        helpers::MessageSigner,
        model::{Apy, DowngradableApy, Product},
    },
};

pub const PRINCIPAL: u128 = 1_000_000;

pub fn admin() -> AccountId {
    "admin".parse().unwrap()
}

impl Jar {
    pub(crate) fn generate(id: u32, account_id: &AccountId, product_id: &str) -> Jar {
        JarV1 {
            id,
            account_id: account_id.clone(),
            product_id: product_id.to_string(),
            created_at: 0,
            principal: 0,
            cache: None,
            claimed_balance: 0,
            is_pending_withdraw: false,
            is_penalty_applied: false,
            claim_remainder: Default::default(),
        }
        .into()
    }

    pub(crate) fn principal(mut self, principal: TokenAmount) -> Jar {
        self.principal = principal;
        self
    }

    pub(crate) fn created_at(mut self, created_at: Timestamp) -> Jar {
        self.created_at = created_at;
        self
    }

    pub(crate) fn pending_withdraw(mut self) -> Jar {
        self.is_pending_withdraw = true;
        self
    }
}

pub fn generate_premium_product(id: &str, signer: &MessageSigner) -> Product {
    Product::generate(id)
        .enabled(true)
        .public_key(signer.public_key())
        .cap(0, 100_000_000_000)
        .apy(Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(20, 2),
            fallback: UDecimal::new(10, 2),
        }))
}

pub fn generate_product(id: &str) -> Product {
    Product::generate(id)
        .enabled(true)
        .cap(0, 100_000_000_000)
        .apy(Apy::Constant(UDecimal::new(20, 2)))
}

pub trait AfterCatchUnwind {
    fn after_catch_unwind(&self);
}

impl AfterCatchUnwind for () {
    fn after_catch_unwind(&self) {}
}

pub fn expect_panic(ctx: &impl AfterCatchUnwind, msg: &str, action: impl FnOnce() + UnwindSafe) {
    let res = catch_unwind(move || action());

    let panic_msg = res.err().expect(&format!(
        "Contract didn't panic when expected to.\nExpected message: {msg}"
    ));

    let panic_msg = panic_msg
        .downcast_ref::<String>()
        .expect(&format!("Contract didn't panic with String.\nExpected message: {msg}"));

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
