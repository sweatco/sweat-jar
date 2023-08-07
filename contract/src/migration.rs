use near_sdk::{AccountId, near_bindgen, require};
use near_sdk::__private::schemars::Set;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use crate::common::{Timestamp, TokenAmount};
use crate::*;
use crate::event::{emit, EventKind, MigrationEventItem};
use crate::product::ProductId;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct CeFiJar {
    pub id: String,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub principal: TokenAmount,
    pub created_at: Timestamp,
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn migrate_jars(&mut self, jars: Vec<CeFiJar>, total_received: U128) {
        let mut event_data: Vec<MigrationEventItem> = vec![];
        let mut total_amount: TokenAmount = 0;

        let product_ids: Set<ProductId> = self.products.keys()
            .cloned()
            .collect();

        for ce_fi_jar in jars {
            require!(
                product_ids.contains(&ce_fi_jar.product_id), 
                format!("Product {} is not registered", ce_fi_jar.product_id),
            );

            let index = self.jars.len();

            let jar = Jar {
                index,
                account_id: ce_fi_jar.account_id,
                product_id: ce_fi_jar.product_id,
                created_at: ce_fi_jar.created_at,
                principal: ce_fi_jar.principal,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                state: JarState::Active,
                is_penalty_applied: false,
            };

            self.jars.push(jar.clone());

            let mut account_jars = self.account_jars.get(&jar.account_id)
                .map_or_else(
                    || HashSet::new(),
                    |result| result.clone(),
                );
            account_jars.insert(jar.index);
            self.account_jars.insert(jar.clone().account_id, account_jars);

            total_amount += jar.principal;

            event_data.push(
                MigrationEventItem {
                    original_id: ce_fi_jar.id,
                    index: jar.index,
                    account_id: jar.account_id,
                }
            );
        }

        assert_eq!(total_received.0, total_amount, "Total received doesn't match the sum of principals");

        emit(EventKind::Migration(event_data));
    }
}