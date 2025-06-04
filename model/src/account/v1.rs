use std::collections::HashMap;

use near_sdk::{env, env::panic_str, near};

use crate::{
    jar::{Deposit, Jar},
    ProductId, Score, Timestamp, Timezone, TokenAmount, UTC,
};

#[near]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountV1 {
    pub nonce: u32,
    pub jars: HashMap<ProductId, Jar>,
    pub score: AccountScore,
    pub is_penalty_applied: bool,
}

const DAYS_STORED: usize = 2;

#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AccountScore {
    pub updated: UTC,
    pub timezone: Timezone,
    /// Scores buffer used for interest calculation. Can be invalidated on claim.
    pub scores: [Score; DAYS_STORED],
    /// Score history values used for displaying it in application. Will not be invalidated during claim.
    pub scores_history: [Score; DAYS_STORED],
}

impl Default for AccountScore {
    fn default() -> Self {
        Self {
            updated: env::block_timestamp_ms().into(),
            timezone: Timezone::invalid(),
            scores: [0, 0],
            scores_history: [0, 0],
        }
    }
}

impl AccountV1 {
    pub fn deposit(&mut self, product_id: &ProductId, principal: TokenAmount, time: Option<Timestamp>) -> &mut Jar {
        let deposit = Deposit {
            created_at: time.unwrap_or_else(env::block_timestamp_ms),
            principal,
        };
        let jar = self.jars.entry(product_id.clone()).or_default();
        jar.deposits.push(deposit);

        jar
    }

    pub fn get_jar_mut(&mut self, product_id: &ProductId) -> &mut Jar {
        self.jars
            .get_mut(product_id)
            .unwrap_or_else(|| panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }
}
