use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::Balance;
use rust_decimal::Decimal;

use crate::common::{Duration, UDecimal};

const SECONDS_IN_YEAR: Duration = 365 * 24 * 60 * 60;

pub type ProductId = String;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct Product {
    pub id: ProductId,
    pub lockup_term: Duration,
    pub maturity_term: Option<Duration>,
    pub notice_term: Option<Duration>,
    pub apy: UDecimal,
    pub cap: Balance,
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

impl Product {
    pub(crate) fn per_second_interest_rate(&self) -> UDecimal {
        let apy = Decimal::new(self.apy.significand as _, self.apy.exponent);
        let per_second_rate = apy
            .checked_div(Decimal::new(SECONDS_IN_YEAR as _, 0))
            .expect("Division error");
        
        UDecimal {
            significand: per_second_rate.mantissa() as _,
            exponent: per_second_rate.scale(),
        }
    }
}

pub trait ProductApi {
    fn register_product(&mut self, product: Product);
    fn get_products(&self) -> Vec<Product>;
}
