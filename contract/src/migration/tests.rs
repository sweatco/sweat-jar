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
