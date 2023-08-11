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
    pub apy: Apy,
    // TODO: check that remaining balance is more than cap.min on partial withdraw
    pub cap: Cap,
    pub terms: Terms,
    // TODO: check that amount to withdraw is more that fee on withdraw
    pub withdrawal_fee: Option<WithdrawalFee>,
    pub public_key: Option<Vec<u8>>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum Terms {
    Fixed(FixedProductTerms),
    Flexible,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct FixedProductTerms {
    pub lockup_term: Duration,
    pub allows_top_up: bool,
    pub allows_restaking: bool,
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
        self.terms == Terms::Flexible
    }

    pub(crate) fn allows_top_up(&self) -> bool {
        match self.clone().terms {
            Terms::Fixed(value) => value.allows_top_up,
            Terms::Flexible => true
        }
    }

    pub(crate) fn allows_restaking(&self) -> bool {
        match self.clone().terms {
            Terms::Fixed(value) => value.allows_restaking,
            Terms::Flexible => false
        }
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

#[cfg(test)]
impl Product {
    pub(crate) fn get_lockup_term(&self) -> Option<Duration> {
        match self.clone().terms {
            Terms::Fixed(value) => Some(value.lockup_term),
            Terms::Flexible => None,
        }
    }
}