use std::collections::HashMap;

use near_sdk::{
    json_types::{U128, U64},
    near, AccountId, Timestamp,
};

use crate::{numbers::U32, ProductId, TokenAmount};

pub type JarId = u32;

pub type JarIdView = U32;

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct JarView {
    pub id: JarIdView,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
    pub claimed_balance: U128,
    pub is_penalty_applied: bool,
    #[serde(default)]
    pub is_pending_withdraw: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[near(serializers=[json])]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<JarIdView, U128>,
    pub total: U128,
}

impl Default for AggregatedTokenAmountView {
    fn default() -> Self {
        Self {
            detailed: HashMap::default(),
            total: U128(0),
        }
    }
}

#[derive(Debug, PartialEq)]
#[near(serializers=[json])]
pub struct AggregatedInterestView {
    pub amount: AggregatedTokenAmountView,
    pub timestamp: Timestamp,
}

#[near(serializers=[json])]
pub struct CeFiJar {
    pub id: String,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub principal: U128,
    pub created_at: U64,
}

// v2
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct Jar {
    pub deposits: Vec<Deposit>,
    pub cache: Option<JarCache>,
    pub is_pending_withdraw: bool,
    pub claim_remainder: u64,
}

impl Jar {
    pub fn add_to_cache(&mut self, now: Timestamp, amount: TokenAmount, remainder: u64) {
        let mut cache = self.cache.unwrap_or_default();
        cache.interest += amount;
        cache.updated_at = now;

        self.cache = cache.into();
        self.claim_remainder += remainder;
    }
}

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Deposit {
    pub created_at: Timestamp,
    pub principal: u128,
}

#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: u128,
}
