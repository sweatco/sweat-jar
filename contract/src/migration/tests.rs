#![cfg(test)]

use near_sdk::{borsh::BorshDeserialize, test_utils::test_env::alice};
use sweat_jar_model::product::Product;

use crate::{
    common::tests::Context,
    jar::{
        account::Account,
        model::{AccountLegacyV3, AccountLegacyV3Wrapper, JarLegacyV1, JarVersionedLegacy},
    },
    test_utils::admin,
};

#[test]
fn deserialize_transferred_account() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let mut context = Context::new(admin).with_products(&[product.clone()]);

    let principal_1 = 1_000_000_000;
    let principal_2 = 5_000_000;

    let account = AccountLegacyV3 {
        last_id: 0,
        jars: vec![
            JarVersionedLegacy::V1(JarLegacyV1 {
                id: 0,
                account_id: alice.clone(),
                product_id: product.id.clone(),
                created_at: 0,
                principal: principal_1,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            }),
            JarVersionedLegacy::V1(JarLegacyV1 {
                id: 1,
                account_id: alice.clone(),
                product_id: product.id.clone(),
                created_at: 0,
                principal: principal_2,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            }),
        ],
        score: Default::default(),
    };
    context
        .contract()
        .archive
        .accounts_v3
        .insert(alice.clone(), AccountLegacyV3Wrapper::V1(account));

    context.switch_account(alice);
    let (account_b64, balance, memo) = context.contract().prepare_migration_data();
    let account_bytes: Vec<u8> = account_b64.into();

    let account: Account = BorshDeserialize::try_from_slice(&account_bytes).unwrap();
    assert_eq!(1, account.jars.len());
    assert_eq!(principal_1 + principal_2, account.get_total_principal());
}

#[test]
fn store_transferred_account() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let mut context = Context::new(admin).with_products(&[product.clone()]);

    let principal_1 = 1_000_000_000;
    let principal_2 = 5_000_000;

    let account = AccountLegacyV3 {
        last_id: 0,
        jars: vec![
            JarVersionedLegacy::V1(JarLegacyV1 {
                id: 0,
                account_id: alice.clone(),
                product_id: product.id.clone(),
                created_at: 0,
                principal: principal_1,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            }),
            JarVersionedLegacy::V1(JarLegacyV1 {
                id: 1,
                account_id: alice.clone(),
                product_id: product.id.clone(),
                created_at: 0,
                principal: principal_2,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            }),
        ],
        score: Default::default(),
    };
    context
        .contract()
        .archive
        .accounts_v3
        .insert(alice.clone(), AccountLegacyV3Wrapper::V1(account));

    context.switch_account(alice.clone());
    let (account_b64, balance, memo) = context.contract().prepare_migration_data();
    context.contract().archive.accounts_v3.remove(&alice);

    context.contract().store_account(alice.clone(), account_b64);

    let contract = context.contract();
    let account = contract.get_account(&alice);
    assert_eq!(1, account.jars.len());
    assert_eq!(principal_1 + principal_2, account.get_total_principal());
}
