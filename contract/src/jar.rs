use std::cmp;

use near_sdk::{AccountId, Balance};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use crate::common::{MS_IN_MINUTE, Timestamp};
use crate::product::{Apy, per_minute_interest_rate, Product, ProductId};

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
            println!("@@ now = {}, maturity at = {}", now, self.created_at + maturity_term);
            cmp::min(now, self.created_at + maturity_term)
        } else {
            now
        };

        let rate_per_minute = per_minute_interest_rate(self.get_apy(product)) as f64;
        let term = ((until_date - base_date) / MS_IN_MINUTE) as f64;
        let interest = (self.principal as f64 * rate_per_minute * term).round() as u128;

        base_interest + interest
    }

    fn get_apy(&self, product: &Product) -> f32 {
        match product.apy.clone() {
            Apy::Constant(apy) => apy,
            Apy::Downgradable(apy) => if self.is_penalty_applied {
                apy.fallback
            } else {
                apy.default
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::AccountId;
    use crate::jar::Jar;
    use crate::product::{Apy, Cap, Product};

    fn get_product() -> Product {
        Product {
            id: "product".to_string(),
            lockup_term: 365 * 24 * 60 * 60 * 1000,
            maturity_term: Some(365 * 24 * 60 * 60 * 1000),
            notice_term: None,
            is_refillable: false,
            apy: Apy::Constant(0.12),
            cap: Cap {
                min: 100,
                max: 100_000_000_000,
            },
            is_restakable: false,
            withdrawal_fee: None,
            public_key: None,
        }
    }

    #[test]
    fn get_interest_before_maturity() {
        let product = get_product();
        let jar = Jar::create(0, AccountId::new_unchecked("alice".to_string()), product.clone().id, 100_000_000, 0);

        let interest = jar.get_interest(&product, 365 * 24 * 60 * 60 * 1000);
        assert_eq!(12_000_000, interest);
    }

    #[test]
    fn get_interest_after_maturity() {
        let product = get_product();
        let jar = Jar::create(0, AccountId::new_unchecked("alice".to_string()), product.clone().id, 100_000_000, 0);

        let interest = jar.get_interest(&product, 400 * 24 * 60 * 60 * 1000);
        assert_eq!(12_000_000, interest);
    }
}
