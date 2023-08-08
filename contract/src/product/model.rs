use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::common::{Duration, TokenAmount, UDecimal};

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
pub enum WithdrawalFee {
    Fix(TokenAmount),
    Percent(UDecimal),
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
pub struct DowngradableApy {
    pub default: UDecimal,
    pub fallback: UDecimal,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct Cap {
    pub min: TokenAmount,
    pub max: TokenAmount,
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