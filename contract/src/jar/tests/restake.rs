use near_sdk::test_utils::test_env::{alice, bob, carol};
use sweat_jar_model::{
    api::{JarApi, ProductApi, RestakeApi},
    MS_IN_DAY, MS_IN_YEAR,
};

use crate::{
    common::tests::Context,
    jar::model::JarV2,
    product::model::ProductV2,
    test_utils::{admin, expect_panic},
};

#[test]
fn restake_by_not_owner() {
    let product = ProductV2::new();
    let alice_jar = JarV2::new();
    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), alice_jar.clone())]);

    context.switch_account(bob());
    expect_panic(&context, "Account bob.near is not found", || {
        context.contract().restake(product.id.clone());
    });

    expect_panic(&context, "Account bob.near is not found", || {
        context.contract().restake_all("some_product".to_string(), None);
    });

    context.switch_account(carol());
    expect_panic(&context, "Account carol.near is not found", || {
        context.contract().restake(product.id);
    });

    expect_panic(&context, "Account carol.near is not found", || {
        context.contract().restake_all("some_product".to_string(), None);
    });
}

#[test]
#[should_panic(expected = "Nothing to restake")]
fn restake_before_maturity() {
    let alice = alice();
    let admin = admin();

    let product = ProductV2::new();
    let alice_jar = JarV2::new();
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    context.switch_account(&alice);
    context.contract().restake(product.id);
}

#[test]
#[should_panic(expected = "The product is disabled")]
fn restake_with_disabled_product() {
    let alice = alice();
    let admin = admin();

    let product = ProductV2::new();
    let alice_jar = JarV2::new();
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| context.contract().set_enabled(product.id.clone(), false));

    context.contract().products_cache.borrow_mut().clear();

    context.set_block_timestamp_in_days(366);

    context.switch_account(&alice);
    context.contract().restake(product.id);
}

#[test]
#[should_panic(expected = "Nothing to restake")]
fn restake_empty_jar() {
    let alice = alice();
    let admin = admin();

    let product = ProductV2::new();
    let alice_jar = JarV2::new();
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    context.set_block_timestamp_in_days(366);

    context.switch_account(&alice);
    context.contract().restake(product.id);
}

#[test]
fn restake_after_maturity() {
    let alice = alice();
    let admin = admin();

    let product = ProductV2::new();
    let principal = 1_000_000;
    let alice_jar = JarV2::new().with_deposit(0, principal);
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context.switch_account(&alice);
    context.contract().restake(product.id.clone());

    let alice_jars = context.contract().get_jars_for_account(alice);
    assert_eq!(1, alice_jars.len());

    let jar = alice_jars.first().unwrap();
    assert_eq!(principal, jar.principal.0);
    assert_eq!(restake_time, jar.created_at.0);
}
