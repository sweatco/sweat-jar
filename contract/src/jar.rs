use std::cmp;

use near_sdk::{AccountId, env, near_bindgen, require};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U64;
use near_sdk::serde::{Deserialize, Serialize};
use crate::common::{u128_dec_format, MINUTES_IN_YEAR, UDecimal};

use crate::*;
use crate::common::{MS_IN_MINUTE, Timestamp, TokenAmount};
use crate::event::{emit, EventKind};
use crate::product::{Apy, Product, ProductId};

pub type JarIndex = u32;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct Jar {
    pub index: JarIndex,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: Timestamp,
    #[serde(with = "u128_dec_format")]
    pub principal: TokenAmount,
    pub cache: Option<JarCache>,
    #[serde(with = "u128_dec_format")]
    pub claimed_balance: TokenAmount,
    pub is_pending_withdraw: bool,
    pub state: JarState,
    pub is_penalty_applied: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct JarCache {
    pub updated_at: Timestamp,
    #[serde(with = "u128_dec_format")]
    pub interest: TokenAmount,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum JarState {
    Active,
    Closed,
}

pub trait JarApi {
    fn create_jar(
        &mut self,
        account_id: AccountId,
        product_id: ProductId,
        amount: TokenAmount,
        signature: Option<String>,
    ) -> Jar;

    fn restake(&mut self, jar_index: JarIndex) -> Jar;

    fn top_up(&mut self, jar_index: JarIndex, amount: TokenAmount) -> TokenAmount;

    fn get_jar(&self, jar_index: JarIndex) -> Jar;
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<Jar>;

    fn get_total_principal(&self, account_id: AccountId) -> u128;
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> u128;

    fn get_total_interest(&self, account_id: AccountId) -> u128;
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> u128;
}

impl Jar {
    pub(crate) fn create(
        index: JarIndex,
        account_id: AccountId,
        product_id: ProductId,
        principal: TokenAmount,
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
            state: JarState::Active,
            is_penalty_applied: false,
        }
    }

    pub(crate) fn locked(&self) -> Self {
        Self {
            is_pending_withdraw: true,
            ..self.clone()
        }
    }

    pub(crate) fn unlocked(&self) -> Self {
        Self {
            is_pending_withdraw: false,
            ..self.clone()
        }
    }

    pub(crate) fn with_penalty_applied(&self, is_applied: bool) -> Self {
        Self {
            is_penalty_applied: is_applied,
            ..self.clone()
        }
    }

    pub(crate) fn topped_up(&self, amount: TokenAmount, product: &Product, now: Timestamp) -> Self {
        let current_interest = self.get_interest(product, now.0);
        Self {
            principal: self.principal + amount,
            cache: Some(JarCache {
                updated_at: now,
                interest: current_interest,
            }),
            ..self.clone()
        }
    }

    pub(crate) fn claimed(
        &self,
        available_yield: TokenAmount,
        claimed_amount: TokenAmount,
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

    // TODO: maybe this mutation should be performed before transfer
    pub(crate) fn withdrawn(
        &self,
        product: &Product,
        withdrawn_amount: TokenAmount,
        now: Timestamp,
    ) -> Self {
        let current_interest = self.get_interest(product, now.0);
        let state = get_final_state(product, self, withdrawn_amount);

        Self {
            principal: self.principal - withdrawn_amount,
            cache: Some(JarCache {
                updated_at: now,
                interest: current_interest,
            }),
            state,
            ..self.clone()
        }
    }

    pub(crate) fn is_mature(&self, product: &Product, now_ms: u64) -> bool {
        now_ms - self.created_at.0 > product.lockup_term.0
    }

    pub(crate) fn get_interest(&self, product: &Product, now_ms: u64) -> TokenAmount {
        let (base_date, base_interest) = if let Some(cache) = &self.cache {
            (cache.updated_at.0, cache.interest)
        } else {
            (self.created_at.0, 0)
        };
        let until_date = if product.lockup_term.0 > 0 {
            cmp::min(now_ms, self.created_at.0 + product.lockup_term.0)
        } else {
            now_ms
        };

        let term_in_minutes = ((until_date - base_date) / MS_IN_MINUTE) as u128;
        let apy = self.get_apy(product);
        let total_interest = apy.mul(self.principal);

        let interest = (term_in_minutes * total_interest) / MINUTES_IN_YEAR as u128;

        base_interest + interest
    }

    fn get_apy(&self, product: &Product) -> UDecimal {
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

// TODO: extract private api
#[near_bindgen]
impl JarApi for Contract {
    #[private]
    fn create_jar(
        &mut self,
        account_id: AccountId,
        product_id: ProductId,
        amount: TokenAmount,
        signature: Option<String>,
    ) -> Jar {
        let product = self.get_product(&product_id);
        let cap = product.cap;

        if !self.is_authorized_for_product(&account_id, &product_id, signature) {
            env::panic_str("Signature is invalid");
        }

        if cap.min > amount || amount > cap.max {
            env::panic_str(format!("Amount is out of product bounds: [{}..{}]", cap.min, cap.max).as_str());
        }

        let index = self.jars.len() as JarIndex;
        let now = U64(env::block_timestamp_ms());
        let jar = Jar::create(index, account_id.clone(), product_id.clone(), amount, now);

        self.save_jar(&account_id, &jar);

        emit(EventKind::CreateJar(jar.clone()));

        jar
    }

    fn restake(&mut self, jar_index: JarIndex) -> Jar {
        let jar = self.get_jar(jar_index);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);

        let product = self.get_product(&jar.product_id);

        require!(product.is_restakable, "The product doesn't support restaking");

        let now = U64(env::block_timestamp_ms());
        require!(jar.is_mature(&product, now.0), "The jar is not mature yet");

        let index = self.jars.len() as JarIndex;
        let new_jar = Jar::create(index, jar.account_id.clone(), jar.product_id.clone(), jar.principal, now);
        let withdraw_jar = jar.withdrawn(&product, jar.principal, now);

        self.save_jar(&account_id, &withdraw_jar);
        self.save_jar(&account_id, &new_jar);

        new_jar
    }

    #[private]
    fn top_up(&mut self, jar_index: JarIndex, amount: TokenAmount) -> TokenAmount {
        let jar = self.get_jar(jar_index);
        let product = self.get_product(&jar.product_id);

        assert!(product.is_refillable, "The product doesn't allow top-ups");

        let now = U64(env::block_timestamp_ms());
        let topped_up_jar = jar.topped_up(amount, &product, now);

        self.jars.replace(jar_index, topped_up_jar.clone());

        topped_up_jar.principal
    }

    fn get_jar(&self, index: JarIndex) -> Jar {
        self.jars
            .get(index)
            .map_or_else(
                || env::panic_str(format!("Jar on index {} doesn't exist", index).as_str()),
                |value| value.clone(),
            )
    }

    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<Jar> {
        self.account_jar_ids(&account_id)
            .iter()
            .map(|index| self.get_jar(*index))
            .collect()
    }

    fn get_total_principal(&self, account_id: AccountId) -> u128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_principal(jar_indices)
    }

    // TODO: tests
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> u128 {
        jar_indices
            .iter()
            .map(|index| self.get_jar(*index).principal)
            .sum()
    }

    fn get_total_interest(&self, account_id: AccountId) -> u128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_interest(jar_indices)
    }

    // TODO: tests
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> u128 {
        let now = env::block_timestamp_ms();

        jar_indices
            .iter()
            .map(|index| self.get_jar(*index))
            .map(|jar| jar.get_interest(&self.get_product(&jar.product_id), now))
            .sum()
    }
}

fn get_final_state(product: &Product, original_jar: &Jar, withdrawn_amount: TokenAmount) -> JarState {
    if product.is_flexible() || original_jar.principal - withdrawn_amount > 0 {
        JarState::Active
    } else {
        JarState::Closed
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::AccountId;
    use near_sdk::json_types::U64;

    use crate::jar::Jar;
    use crate::product::tests::get_product;

    #[test]
    fn get_interest_before_maturity() {
        let product = get_product();
        let jar = Jar::create(
            0,
            AccountId::new_unchecked("alice".to_string()),
            product.clone().id,
            100_000_000,
            U64(0),
        );

        let interest = jar.get_interest(&product, 365 * 24 * 60 * 60 * 1000);
        assert_eq!(12_000_000, interest);
    }

    #[test]
    fn get_interest_after_maturity() {
        let product = get_product();
        let jar = Jar::create(
            0,
            AccountId::new_unchecked("alice".to_string()),
            product.clone().id,
            100_000_000,
            U64(0),
        );

        let interest = jar.get_interest(&product, 400 * 24 * 60 * 60 * 1000);
        assert_eq!(12_000_000, interest);
    }
}
