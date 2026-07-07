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

#[test]
fn migrate_access_control_grants_all_roles_and_drops_manager() {
    use near_plugins::AccessControllable;
    use near_sdk::{collections::UnorderedMap, env, store::LookupMap, test_utils::VMContextBuilder, testing_env};

    use crate::{migration::api::OldContract, Contract, StorageKey};

    let owner: AccountId = "owner".to_string().try_into().unwrap();

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

    let contract = Contract::migrate_access_control();

    for role in ["Oracle", "ProductManager", "FeeManager", "Maintainer"] {
        assert!(
            contract.acl_has_role(role.to_string(), old_manager.clone()),
            "expected old manager to hold role {role}"
        );
    }
    assert!(contract.acl_is_super_admin(env::current_account_id()));
}
