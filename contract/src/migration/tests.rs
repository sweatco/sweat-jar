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

    context.switch_account_to_manager();
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

#[test]
fn migrate_access_control_grants_all_roles_and_drops_manager() {
    use near_plugins::AccessControllable;
    use near_sdk::{collections::UnorderedMap, env, store::LookupMap, test_utils::VMContextBuilder, testing_env};
    use sweat_jar_model::api::RoleAssignments;

    use crate::{migration::api::OldContract, Contract, StorageKey};

    let owner: AccountId = "owner".to_string().try_into().unwrap();
    let super_admin: AccountId = "super_admin".to_string().try_into().unwrap();
    let operator: AccountId = "operator".to_string().try_into().unwrap();

    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(owner.clone())
        .signer_account_id(owner.clone())
        .predecessor_account_id(owner.clone())
        .block_timestamp(0);
    testing_env!(builder.build());

    let old_manager: AccountId = "old_manager".to_string().try_into().unwrap();

    let old_state = OldContract {
        token_account_id: "token".to_string().try_into().unwrap(),
        fee_account_id: "fee".to_string().try_into().unwrap(),
        manager: old_manager.clone(),
        products: UnorderedMap::new(StorageKey::Products),
        accounts: LookupMap::new(StorageKey::Accounts),
        fee_amount: 0,
        previous_version_account_id: "legacy_jar".to_string().try_into().unwrap(),
    };
    env::state_write(&old_state);

    let roles = RoleAssignments {
        oracle: vec![operator.clone()],
        product_manager: vec![operator.clone()],
        fee_manager: vec![operator.clone()],
        maintainer: vec![operator.clone()],
        staging_manager: vec![operator.clone()],
        upgrade_manager: vec![operator.clone()],
    };

    let contract = Contract::migrate_access_control(super_admin.clone(), roles);

    for role in [
        "Oracle",
        "ProductManager",
        "FeeManager",
        "Maintainer",
        "StagingManager",
        "UpgradeManager",
    ] {
        assert!(
            contract.acl_has_role(role.to_string(), operator.clone()),
            "expected operator to hold role {role}"
        );
        assert!(
            !contract.acl_has_role(role.to_string(), old_manager.clone()),
            "old manager should not automatically hold role {role} anymore"
        );
    }
    assert!(contract.acl_is_super_admin(super_admin));
    assert!(!contract.acl_is_super_admin(env::current_account_id()));
}
