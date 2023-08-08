use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::product::model::{Cap, DowngradableApy, WithdrawalFee};

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