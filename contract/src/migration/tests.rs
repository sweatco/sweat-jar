#![cfg(test)]

use near_sdk::AccountId;
use rstest::rstest;
use sweat_jar_model::data::product::Product;

use crate::{
    common::{
        event::EventKind,
        testing::{accounts::*, Context},
    },
    feature::product::model::test_utils::*,
};

#[rstest]
fn migrate_products_by_authorized_account(
    #[from(admin)] admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product_1: Product,
    #[from(product_1_year_apy_20_percent)] product_2: Product,
) {
    let mut context = Context::new(admin);

    let previous_version_account_id = context.contract().previous_version_account_id.clone();
    context.switch_account(&previous_version_account_id);
    context
        .contract()
        .migrate_products(vec![product_1.clone(), product_2.clone()]);

    assert_eq!(2, context.contract().products.len());

    let events = context.get_events();
    let EventKind::MigrateProducts(product_ids) = events.last().unwrap() else {
        panic!("Expected MigrateProducts event");
    };

    assert_eq!(2, product_ids.len());
    assert!(product_ids.contains(&product_1.id));
    assert!(product_ids.contains(&product_2.id));
}

#[rstest]
#[should_panic(expected = "Can migrate data only from previous version")]
fn migrate_products_by_unauthorized_account(
    #[from(admin)] admin: AccountId,
    #[from(alice)] alice: AccountId,
    #[from(product_1_year_apy_10_percent)] product_1: Product,
    #[from(product_1_year_apy_20_percent)] product_2: Product,
) {
    let mut context = Context::new(admin);

    context.switch_account(&alice);
    context.contract().migrate_products(vec![product_1, product_2]);
}

#[rstest]
#[should_panic(expected = "Product already exists")]
fn migrate_products_rejects_existing_product(
    #[from(admin)] admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
) {
    let mut context = Context::new(admin).with_products(&[product.clone()]);

    let previous_version_account_id = context.contract().previous_version_account_id.clone();
    context.switch_account(&previous_version_account_id);
    context.contract().migrate_products(vec![product]);
}

#[rstest]
#[should_panic(expected = "Cap minimum must be less than maximum")]
fn migrate_products_rejects_invalid_cap_order(
    #[from(admin)] admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
) {
    let mut context = Context::new(admin);
    let invalid_product = product.with_cap(1_000, 100);

    let previous_version_account_id = context.contract().previous_version_account_id.clone();
    context.switch_account(&previous_version_account_id);
    context.contract().migrate_products(vec![invalid_product]);
}

#[rstest]
#[should_panic(expected = "Insufficient permissions")]
fn disable_migration_by_non_maintainer_panics(admin: AccountId, alice: AccountId) {
    let mut context = Context::new(admin);

    context.switch_account(&alice);
    context.with_deposit_yocto(1, |context| context.contract().disable_migration());
}

#[rstest]
fn disable_migration_blocks_further_migration(
    #[from(admin)] admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
) {
    let mut context = Context::new(admin);
    let previous_version_account_id = context.contract().previous_version_account_id.clone();

    context.switch_account_to_operator();
    context.with_deposit_yocto(1, |context| context.contract().disable_migration());

    assert_ne!(
        previous_version_account_id,
        context.contract().previous_version_account_id
    );

    let events = context.get_events();
    assert!(matches!(events.last().unwrap(), EventKind::MigrationDisabled));

    context.switch_account(&previous_version_account_id);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        context.contract().migrate_products(vec![product]);
    }));
    assert!(result.is_err(), "migrate_products must fail after disable_migration");
}

#[rstest]
#[should_panic(expected = "Insufficient permissions")]
fn enable_migration_by_non_maintainer_panics(admin: AccountId, alice: AccountId) {
    let mut context = Context::new(admin);
    let previous_version_account_id = context.contract().previous_version_account_id.clone();

    context.switch_account(&alice);
    context.with_deposit_yocto(1, |context| {
        context.contract().enable_migration(previous_version_account_id);
    });
}

#[rstest]
fn enable_migration_reopens_migration_after_disable(
    #[from(admin)] admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
) {
    let mut context = Context::new(admin);
    let previous_version_account_id = context.contract().previous_version_account_id.clone();

    context.switch_account_to_operator();
    context.with_deposit_yocto(1, |context| context.contract().disable_migration());

    let restored_id = previous_version_account_id.clone();
    context.with_deposit_yocto(1, |context| context.contract().enable_migration(restored_id));

    assert_eq!(
        previous_version_account_id,
        context.contract().previous_version_account_id
    );

    let events = context.get_events();
    let EventKind::MigrationEnabled(enabled_for) = events.last().unwrap() else {
        panic!("Expected MigrationEnabled event");
    };
    assert_eq!(&previous_version_account_id, enabled_for);

    context.switch_account(&previous_version_account_id);
    context.contract().migrate_products(vec![product]);
    assert_eq!(1, context.contract().products.len());
}
