#![cfg(test)]

use std::panic::{catch_unwind, UnwindSafe};

use near_sdk::AccountId;

use crate::common::tests::Context;

pub fn admin() -> AccountId {
    "admin".parse().unwrap()
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
