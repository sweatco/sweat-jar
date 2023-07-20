use near_sdk::{AccountId, near_bindgen};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use crate::common::{Timestamp, TokenAmount};
use crate::common::{u64_dec_format, u128_dec_format};
use crate::*;
use crate::event::{emit, EventKind, MigrationEventItem};
use crate::jar::JarCache;
use crate::product::ProductId;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct CeFiJar {
    pub id: String,
    pub account_id: AccountId,
    pub product_id: ProductId,
    #[serde(with = "u128_dec_format")]
    pub principal: TokenAmount,
    #[serde(with = "u64_dec_format")]
    pub created_at: Timestamp,
    pub claim: Option<CeFiJarClaim>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct CeFiJarClaim {
    #[serde(with = "u128_dec_format")]
    pub claimed_amount: TokenAmount,
    #[serde(with = "u64_dec_format")]
    pub last_claim_at: Timestamp,
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn migrate_jars(&mut self, jars: Vec<CeFiJar>, total_received: TokenAmount) {
        let mut event_data: Vec<MigrationEventItem> = vec![];
        let mut total_amount: TokenAmount = 0;

        for ce_fi_jar in jars {
            let index = self.jars.len();
            let cache = ce_fi_jar.claim.map(|claim| JarCache {
                updated_at: claim.last_claim_at,
                interest: claim.claimed_amount,
            });

            let jar = Jar {
                index,
                account_id: ce_fi_jar.account_id,
                product_id: ce_fi_jar.product_id,
                created_at: ce_fi_jar.created_at,
                principal: ce_fi_jar.principal,
                cache: cache.clone(),
                claimed_balance: cache.clone().map_or(0, |value| value.interest),
                is_pending_withdraw: false,
                state: JarState::Active,
                is_penalty_applied: false,
            };

            self.jars.push(&jar);

            let mut account_jars = self.account_jars.get(&jar.account_id).unwrap_or(HashSet::new());
            account_jars.insert(jar.index);
            self.account_jars.insert(&jar.account_id, &account_jars);

            total_amount += jar.principal;

            event_data.push(
                MigrationEventItem {
                    original_id: ce_fi_jar.id,
                    index: jar.index,
                    account_id: jar.account_id,
                }
            );
        }

        assert_eq!(total_received, total_amount, "Total received doesn't match the sum of principals");

        emit(EventKind::Migration(event_data));
    }
}