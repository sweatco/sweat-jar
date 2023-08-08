use near_sdk::near_bindgen;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::common::{Duration, TokenAmount, UDecimal};
use crate::event::{emit, EventKind};

pub type ProductId = String;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct Product {
    pub id: ProductId,
    pub lockup_term: Duration,
    pub apy: Apy,
    pub cap: Cap,
    pub is_refillable: bool,
    pub is_restakable: bool,
    pub withdrawal_fee: Option<WithdrawalFee>,
    pub public_key: Option<Vec<u8>>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct ProductView {
    pub id: ProductId,
    pub lockup_term: U64,
    pub apy: ApyView,
    pub cap: CapView,
    pub is_refillable: bool,
    pub is_restakable: bool,
    pub withdrawal_fee: Option<WithdrawalFeeView>,
}

impl From<Product> for ProductView {
    fn from(value: Product) -> Self {
        Self {
            id: value.id,
            lockup_term: U64(value.lockup_term),
            apy: value.apy.into(),
            cap: value.cap.into(),
            is_refillable: value.is_refillable,
            is_restakable: value.is_restakable,
            withdrawal_fee: value.withdrawal_fee.map(|fee| fee.into()),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub enum WithdrawalFee {
    Fix(TokenAmount),
    Percent(UDecimal),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub enum WithdrawalFeeView {
    Fix(U128),
    Percent(f32),
}

impl From<WithdrawalFee> for WithdrawalFeeView {
    fn from(value: WithdrawalFee) -> Self {
        match value {
            WithdrawalFee::Fix(value) => WithdrawalFeeView::Fix(U128(value)),
            WithdrawalFee::Percent(value) => WithdrawalFeeView::Percent(value.to_f32())
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub enum Apy {
    Constant(UDecimal),
    Downgradable(DowngradableApy),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub enum ApyView {
    Constant(f32),
    Downgradable(DowngradableApyView),
}

impl From<Apy> for ApyView {
    fn from(value: Apy) -> Self {
        match value {
            Apy::Constant(value) => ApyView::Constant(value.to_f32()),
            Apy::Downgradable(value) => ApyView::Downgradable(value.into())
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct DowngradableApy {
    pub default: UDecimal,
    pub fallback: UDecimal,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct DowngradableApyView {
    pub default: f32,
    pub fallback: f32,
}

impl From<DowngradableApy> for DowngradableApyView {
    fn from(value: DowngradableApy) -> Self {
        Self {
            default: value.default.to_f32(),
            fallback: value.fallback.to_f32(),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct Cap {
    pub min: TokenAmount,
    pub max: TokenAmount,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct CapView {
    pub min: U128,
    pub max: U128,
}

impl From<Cap> for CapView {
    fn from(value: Cap) -> Self {
        Self {
            min: U128(value.min),
            max: U128(value.max),
        }
    }
}

impl Product {
    pub(crate) fn is_flexible(&self) -> bool {
        self.lockup_term == 0
    }

    pub(crate) fn assert_cap(&self, amount: TokenAmount) {
        if self.cap.min > amount || amount > self.cap.max {
            env::panic_str(format!(
                "Total amount is out of product bounds: [{}..{}]",
                self.cap.min,
                self.cap.max
            ).as_str());
        }
    }
}

pub trait ProductApi {
    fn register_product(&mut self, product: Product);
    fn get_products(&self) -> Vec<ProductView>;
}

#[near_bindgen]
impl ProductApi for Contract {
    fn register_product(&mut self, product: Product) {
        self.assert_admin();

        self.products.insert(product.clone().id, product.clone());

        emit(EventKind::RegisterProduct(product));
    }

    fn get_products(&self) -> Vec<ProductView> {
        self.products.values().map(|product| product.clone().into()).collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::common::UDecimal;
    use crate::product::{Apy, Cap, DowngradableApy, Product};

    pub(crate) fn get_product() -> Product {
        Product {
            id: "product".to_string(),
            lockup_term: 365 * 24 * 60 * 60 * 1000,
            is_refillable: false,
            apy: Apy::Constant(UDecimal::new(12, 2)),
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
            is_refillable: false,
            apy: Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }),
            cap: Cap {
                min: 100,
                max: 100_000_000_000,
            },
            is_restakable: false,
            withdrawal_fee: None,
            public_key: Some(vec![
                33, 80, 163, 149, 64, 30, 150, 45, 68, 212, 97, 122, 213, 118, 189, 174, 239, 109,
                48, 82, 50, 35, 197, 176, 50, 211, 183, 128, 207, 1, 8, 68,
            ]),
        }
    }

    #[test]
    fn assert_cap_in_bounds() {
        get_product().assert_cap(200);
    }

    #[test]
    #[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
    fn assert_cap_less_than_min() {
        get_product().assert_cap(10);
    }

    #[test]
    #[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
    fn assert_cap_more_than_max() {
        get_product().assert_cap(500_000_000_000);
    }
}
