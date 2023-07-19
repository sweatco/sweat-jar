use near_sdk::near_bindgen;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::common::{Duration, MINUTES_IN_YEAR, TokenAmount};
use crate::common::u128_dec_format;
use crate::event::{emit, EventKind};

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
    Fix(
        #[serde(with = "u128_dec_format")]
        TokenAmount
    ),
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
    #[serde(with = "u128_dec_format")]
    pub min: TokenAmount,
    #[serde(with = "u128_dec_format")]
    pub max: TokenAmount,
}

impl Product {
    pub(crate) fn is_flexible(&self) -> bool {
        self.maturity_term.is_none()
    }
}

pub(crate) fn per_minute_interest_rate(rate: f32) -> f32 {
    rate / MINUTES_IN_YEAR as f32
}

pub trait ProductApi {
    fn register_product(&mut self, product: Product);
    fn get_products(&self) -> Vec<Product>;
}

#[near_bindgen]
impl ProductApi for Contract {
    fn register_product(&mut self, product: Product) {
        self.assert_admin();

        self.products.insert(&product.id, &product);

        emit(EventKind::RegisterProduct(product));
    }

    fn get_products(&self) -> Vec<Product> {
        self.products.values_as_vector().to_vec()
    }
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

    pub(crate) fn get_product_with_notice() -> Product {
        Product {
            id: "product_with_notice".to_string(),
            lockup_term: 365 * 24 * 60 * 60 * 1000,
            maturity_term: Some(365 * 24 * 60 * 60 * 1000),
            notice_term: Some(48 * 60 * 60 * 1000),
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
