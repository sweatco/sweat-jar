use std::cmp;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, Balance};

use crate::common::{Timestamp, UDecimal};
use crate::product::{Product, ProductId, Apy, per_second_interest_rate};

pub type JarIndex = u64;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct Jar {
    pub index: JarIndex,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: Timestamp,
    pub principal: Balance,
    pub cache: Option<JarCache>,
    pub claimed_balance: Balance,
    pub is_pending_withdraw: bool,
    pub noticed_at: Option<Timestamp>,
    pub is_penalty_applied: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: Balance,
}

impl Jar {
    pub fn create(
        index: JarIndex,
        account_id: AccountId,
        product_id: ProductId,
        principal: Balance,
        created_at: Timestamp,
    ) -> Self {
        Self {
            index,
            account_id,
            product_id,
            principal,
            created_at,
            cache: None,
            claimed_balance: 0,
            is_pending_withdraw: false,
            noticed_at: None,
            is_penalty_applied: false,
        }
    }

    pub fn locked(&self) -> Self {
        Self {
            is_pending_withdraw: true,
            ..self.clone()
        }
    }

    pub fn unlocked(&self) -> Self {
        Self {
            is_pending_withdraw: false,
            ..self.clone()
        }
    }

    pub fn noticed(&self, noticed_at: Timestamp) -> Self {
        Self {
            noticed_at: Some(noticed_at),
            ..self.clone()
        }
    }

    pub fn with_penalty_applied(&self, is_applied: bool) -> Self {
        Self {
            is_penalty_applied: is_applied,
            ..self.clone()
        }
    }

    pub fn topped_up(&self, amount: Balance, product: &Product, now: Timestamp) -> Self {
        let current_interest = self.get_interest(product, now);
        Self {
            principal: self.principal + amount,
            cache: Some(JarCache {
                updated_at: now,
                interest: current_interest,
            }),
            ..self.clone()
        }
    }

    pub fn claimed(
        &self,
        available_yield: Balance,
        claimed_amount: Balance,
        now: Timestamp,
    ) -> Self {
        Self {
            claimed_balance: self.claimed_balance + claimed_amount,
            cache: Some(JarCache {
                updated_at: now,
                interest: available_yield - claimed_amount,
            }),
            ..self.clone()
        }
    }

    pub fn get_interest(&self, product: &Product, now: Timestamp) -> Balance {
        let (base_date, base_interest) = if let Some(cache) = &self.cache {
            (cache.updated_at, cache.interest)
        } else {
            (self.created_at, 0)
        };
        let until_date = if let Some(maturity_term) = product.maturity_term {
            cmp::min(now, self.created_at + maturity_term)
        } else {
            now
        };

        let apy = product.apy.clone();
        let rate = match apy {
            Apy::Constant(apy) => apy,
            Apy::Downgradable(apy) => if self.is_penalty_applied {
                apy.fallback
            } else {
                apy.default
            },
        };
        let rate_per_second = per_second_interest_rate(rate);

        let term = (until_date - base_date) / 1000;
        let rate = UDecimal {
            significand: rate_per_second.significand * self.principal,
            ..rate_per_second
        };
        let interest = (rate.significand * term as u128) / (10_u128.pow(rate.exponent as _));

        base_interest + interest
    }
}
