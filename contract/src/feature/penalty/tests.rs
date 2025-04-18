#![cfg(test)]

use near_sdk::AccountId;
use rstest::rstest;
use sweat_jar_model::{
    api::{AccountApi, PenaltyApi},
    data::jar::Jar,
    MS_IN_YEAR,
};

use crate::{
    common::testing::{accounts::*, Context},
    feature::{account::model::test_utils::jar, product::model::test_utils::*},
};

#[rstest]
fn apply_penalty_in_batch(
    admin: AccountId,
    alice: AccountId,
    bob: AccountId,
    #[from(product_1_year_apy_downgradable_20_10_percent_protected)]
    ProtectedProduct { product, signer: _ }: ProtectedProduct,
    #[from(jar)]
    #[with(vec![(0, 10_000_000_000)])]
    alice_jar: Jar,
    #[from(jar)]
    #[with(vec![(0, 5_000_000_000)])]
    bob_jar: Jar,
) {
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar)])
        .with_jars(&bob, &[(product.id.clone(), bob_jar)]);

    context.set_block_timestamp_in_ms(MS_IN_YEAR / 2);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_000_000_000);

    let interest = context.contract().get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 500_000_000);

    context.switch_account(&admin);

    context
        .contract()
        .batch_set_penalty(vec![alice.clone(), bob.clone()], true);

    context.set_block_timestamp_in_days(365);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_500_000_000);

    let interest = context.contract().get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 750_000_000);

    assert!(context.contract().is_penalty_applied(alice));
    assert!(context.contract().is_penalty_applied(bob));
}
