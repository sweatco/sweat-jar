use std::cmp;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, Balance};

const YEAR_IN_SECONDS: u32 = 365 * 24 * 60 * 60;

pub type Timestamp = u64;

pub type Duration = u64;

pub type ProductId = String;

pub type JarIndex = u32;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct Product {
    pub id: ProductId,
    pub lockup_term: Duration,
    pub maturity_term: Duration,
    pub notice_term: Duration,
    pub is_refillable: bool,
    pub apy: f32,
    pub cap: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct Jar {
    pub index: JarIndex,
    pub product_id: ProductId,
    pub stakes: Vec<Stake>,
    pub last_claim_timestamp: Option<Timestamp>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct Stake {
    pub account_id: AccountId,
    pub amount: Balance,
    pub since: Timestamp,
}

impl Jar {
    pub fn get_principal(&self) -> Balance {
        self.stakes.iter().fold(0, |acc, stake| acc + stake.amount)
    }

    pub fn get_intereset(&self, product: Product, now: Timestamp) -> Balance {
        let jar_start_timestamp = self
            .stakes
            .first()
            .expect("Jar must contain at least one stake")
            .since;
        let maturity_date = jar_start_timestamp + product.maturity_term;

        let last_claim_ms = self.last_claim_timestamp.unwrap_or(0);
        let interval_end_ms = cmp::min(now, maturity_date);

        let principal_part = self.stakes.iter().fold(0, |acc, stake| {
            let interval_start_ms = cmp::max(last_claim_ms, stake.since);

            let interval_ms = interval_end_ms - interval_start_ms;

            acc + stake.amount * interval_ms as u128
        });

        (principal_part as f64 * product.apy as f64) as u128 / YEAR_IN_SECONDS as u128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //    #[test]
    //    fn given_jar_with_single_stake_when_get_principle_then_it_equals_to_stake() {
    //        let account_id = AccountId::new_unchecked(String::from("alice"));
    //        let jar = Jar {
    //            stakes: vec![Stake {
    //                account_id,
    //                amount: 100,
    //                since: 0,
    //            }],
    //            last_claim_timestamp: None,
    //        };
    //
    //        assert_eq!(100, jar.get_principal());
    //    }
    //
    //    #[test]
    //    fn given_jar_with_multiple_stakes_when_get_principle_then_it_equals_to_sum() {
    //        let account_id = AccountId::new_unchecked(String::from("alice"));
    //        let jar = Jar {
    //            stakes: vec![
    //                Stake {
    //                    account_id: account_id.clone(),
    //                    amount: 100,
    //                    since: 0,
    //                },
    //                Stake {
    //                    account_id: account_id.clone(),
    //                    amount: 100,
    //                    since: 0,
    //                },
    //                Stake {
    //                    account_id: account_id.clone(),
    //                    amount: 300,
    //                    since: 0,
    //                },
    //            ],
    //            last_claim_timestamp: None,
    //        };
    //
    //        assert_eq!(500, jar.get_principal());
    //    }
    //
    //    #[test]
    //    fn given_new_stake_when_get_principal_then_return_zero() {
    //        let account_id = AccountId::new_unchecked(String::from("alice"));
    //        let jar = Jar {
    //            stakes: vec![Stake {
    //                account_id: account_id.clone(),
    //                amount: 100,
    //                since: 0,
    //            }],
    //            last_claim_timestamp: None,
    //        };
    //
    //        assert_eq!(0, jar.get_intereset(0.05, 1, 0));
    //    }
    //
    //    #[test]
    //    fn given_mature_stake_when_get_principal_then_return_max_interest() {
    //        let account_id = AccountId::new_unchecked(String::from("alice"));
    //        let jar = Jar {
    //            stakes: vec![Stake {
    //                account_id: account_id.clone(),
    //                amount: 100,
    //                since: 0,
    //            }],
    //            last_claim_timestamp: None,
    //        };
    //
    //        assert_eq!(5, jar.get_intereset(0.05, 1, 24 * 60 * 60));
    //    }
}
