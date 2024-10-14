#![cfg(test)]

use std::panic::{catch_unwind, UnwindSafe};

use near_sdk::{test_utils::test_env::alice, AccountId, PromiseOrValue};
use sweat_jar_model::{TokenAmount, UDecimal, MS_IN_YEAR};

use crate::{
    common::Timestamp,
    jar::model::{Jar, JarLastVersion},
    product::{
        helpers::MessageSigner,
        model::{
            v2::{Apy, DowngradableApy, FixedProductTerms, Terms},
            ProductV2,
        },
    },
};

pub const PRINCIPAL: u128 = 1_000_000;

/// Default product name. If product name wasn't specified it will have this name.
pub(crate) const PRODUCT: &str = "product";
pub(crate) const SCORE_PRODUCT: &str = "score_product";

pub fn admin() -> AccountId {
    "admin".parse().unwrap()
}

impl Jar {
    pub(crate) fn new(id: u32) -> Jar {
        JarLastVersion {
            id,
            account_id: alice(),
            product_id: PRODUCT.to_string(),
            created_at: 0,
            principal: 1_000_000,
            cache: None,
            claimed_balance: 0,
            is_pending_withdraw: false,
            is_penalty_applied: false,
            claim_remainder: Default::default(),
        }
        .into()
    }

    pub(crate) fn product_id(mut self, product_id: &str) -> Jar {
        self.product_id = product_id.to_string();
        self
    }

    pub(crate) fn account_id(mut self, account_id: &AccountId) -> Jar {
        self.account_id = account_id.clone();
        self
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

pub fn generate_premium_product(id: &str, signer: &MessageSigner) -> ProductV2 {
    ProductV2::new()
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
    let res = catch_unwind(move || action());

    let panic_msg = res.err().expect(&format!(
        "Contract didn't panic when expected to.\nExpected message: {msg}"
    ));

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
