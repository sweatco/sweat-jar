use near_sdk::{collections::UnorderedMap, env, near, store::LookupMap, AccountId};
use std::collections::HashMap;
use sweat_jar_model::{
    data::{
        account::versioned::AccountVersioned,
        product::{Product, ProductId},
    },
    TokenAmount,
};

use crate::{feature::booster::model::Boosters, Contract, ContractExt, StorageKey};

#[near]
pub struct OldState {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub accounts: LookupMap<AccountId, AccountVersioned>,
    pub fee_amount: TokenAmount,
    pub previous_version_account_id: AccountId,
}

#[near]
impl Contract {
    #[private]
    #[init(ignore_state)]
    pub fn migrate_state() -> Self {
        let old_state: OldState = near_sdk::env::state_read().expect("Failed to read old state");

        Self {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products: old_state.products,
            accounts: old_state.accounts,
            products_cache: HashMap::default().into(),
            fee_amount: old_state.fee_amount,
            previous_version_account_id: old_state.previous_version_account_id,
            boosters: Boosters::new(
                env::block_timestamp_ms(),
                StorageKey::BoostersIndex,
                StorageKey::BoostersItems,
            ),
        }
    }
}
