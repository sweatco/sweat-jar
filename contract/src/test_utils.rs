#![cfg(test)]

use std::panic::{catch_unwind, UnwindSafe};

use near_sdk::AccountId;
use sweat_jar_model::TokenAmount;

use crate::{
    common::{tests::Context, udecimal::UDecimal, Timestamp},
    jar::model::{Jar, JarV1},
    product::{
        helpers::MessageSigner,
        model::{Apy, DowngradableApy, Product},
    },
};

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

pub fn expect_panic(ctx: &Context, msg: &str, action: impl FnOnce(&Context) + UnwindSafe) {
    ctx.before_catch_unwind();

    let res = catch_unwind(move || action(ctx));

    let panic_msg = res.unwrap_err();

    let panic_msg = panic_msg
        .downcast_ref::<String>()
        .expect(&format!("Contract didn't panic. Expected {msg}"));

    assert!(
        panic_msg.contains(msg),
        "Expected panic message to contain: {msg}.\nPanic message: {panic_msg}"
    );
}
