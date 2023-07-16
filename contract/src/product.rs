use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::Balance;

use crate::common::{Duration, MINUTES_IN_YEAR};

pub type ProductId = String;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct Product {
    pub id: ProductId,
    pub lockup_term: Duration,
    pub maturity_term: Option<Duration>,
    pub notice_term: Option<Duration>,
    pub apy: Apy,
    pub cap: Cap,
    pub is_refillable: bool,
    pub is_restakable: bool,
    pub withdrawal_fee: Option<WithdrawalFee>,
    pub public_key: Option<Vec<u8>>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub enum WithdrawalFee {
    Fix(Balance),
    Percent(f32),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub enum Apy {
    Constant(f32),
    Downgradable(DowngradableApy),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct DowngradableApy {
    pub default: f32,
    pub fallback: f32,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct Cap {
    pub min: u128,
    pub max: u128,
}

pub(crate) fn per_minute_interest_rate(rate: f32) -> f32 {
    rate / MINUTES_IN_YEAR as f32
}

pub trait ProductApi {
    fn register_product(&mut self, product: Product);
    fn get_products(&self) -> Vec<Product>;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::product::{Apy, Cap, DowngradableApy, Product};

    pub(crate) fn get_product() -> Product {
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

    pub(crate) fn get_premium_product() -> Product {
        Product {
            id: "product_premium".to_string(),
            lockup_term: 365 * 24 * 60 * 60 * 1000,
            maturity_term: Some(365 * 24 * 60 * 60 * 1000),
            notice_term: None,
            is_refillable: false,
            apy: Apy::Downgradable(DowngradableApy { default: 0.20, fallback: 0.10 }),
            cap: Cap {
                min: 100,
                max: 100_000_000_000,
            },
            is_restakable: false,
            withdrawal_fee: None,
            public_key: Some(vec![
                26, 19, 155, 89, 46, 117, 31, 171, 221, 114, 253, 247, 67, 65, 59, 77, 221, 88, 57,
                24, 102, 211, 115, 9, 238, 50, 221, 246, 161, 94, 210, 116,
            ]),
        }
    }
}
