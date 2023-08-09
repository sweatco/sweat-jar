use std::cmp;

use near_sdk::{AccountId, env, require};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::env::sha256;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::common::{MINUTES_IN_YEAR, UDecimal};
use crate::common::{MS_IN_MINUTE, Timestamp, TokenAmount};
use crate::event::{emit, EventKind};
use crate::jar::view::JarView;
use crate::product::model::{Apy, Product, ProductId};

pub type JarIndex = u32;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct JarTicket {
    pub product_id: String,
    pub valid_until: U64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct Jar {
    pub index: JarIndex,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: Timestamp,
    pub principal: TokenAmount,
    pub cache: Option<JarCache>,
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
    pub interest: TokenAmount,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum JarState {
    Active,
    Closed,
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
        let current_interest = self.get_interest(product, now);
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

    pub(crate) fn is_mature(&self, product: &Product, now: Timestamp) -> bool {
        now - self.created_at > product.lockup_term
    }

    pub(crate) fn get_interest(&self, product: &Product, now: Timestamp) -> TokenAmount {
        let (base_date, base_interest) = if let Some(cache) = &self.cache {
            (cache.updated_at, cache.interest)
        } else {
            (self.created_at, 0)
        };
        let until_date = if product.lockup_term > 0 {
            cmp::min(now, self.created_at + product.lockup_term)
        } else {
            now
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

fn get_final_state(product: &Product, original_jar: &Jar, withdrawn_amount: TokenAmount) -> JarState {
    if product.is_flexible() || original_jar.principal - withdrawn_amount > 0 {
        JarState::Active
    } else {
        JarState::Closed
    }
}

impl Contract {
    pub(crate) fn create_jar(
        &mut self,
        account_id: AccountId,
        ticket: JarTicket,
        amount: U128,
        signature: Option<Base64VecU8>,
    ) -> JarView {
        let amount = amount.0;
        let product_id = ticket.clone().product_id;
        let product = self.get_product(&product_id);

        product.assert_cap(amount);
        self.verify(&account_id, &ticket, signature);

        let index = self.jars.len() as JarIndex;
        let now = env::block_timestamp_ms();
        let jar = Jar::create(index, account_id.clone(), product_id, amount, now);

        self.save_jar(&account_id, &jar);

        emit(EventKind::CreateJar(jar.clone()));

        jar.into()
    }

    pub(crate) fn top_up(&mut self, jar_index: JarIndex, amount: U128) -> U128 {
        let jar = self.get_jar_internal(jar_index);
        let product = self.get_product(&jar.product_id);

        require!(product.is_refillable, "The product doesn't allow top-ups");
        product.assert_cap(jar.principal + amount.0);

        let now = env::block_timestamp_ms();
        let topped_up_jar = jar.topped_up(amount.0, &product, now);

        self.jars.replace(jar_index, topped_up_jar.clone());

        U128(topped_up_jar.principal)
    }

    pub(crate) fn get_jar_internal(&self, index: JarIndex) -> Jar {
        self.jars
            .get(index)
            .map_or_else(
                || env::panic_str(format!("Jar on index {} doesn't exist", index).as_str()),
                |value| value.clone(),
            )
    }

    pub(crate) fn verify(&self, account_id: &AccountId, ticket: &JarTicket, signature: Option<Base64VecU8>) {
        let product = self.get_product(&ticket.product_id);
        if let Some(pk) = product.public_key {
            let signature = signature.expect("Signature is required");
            let last_jar_index = self.account_jars.get(account_id)
                .map_or_else(
                    || 0,
                    |jars| *jars.iter().max().unwrap(),
                );

            let hash = self.get_ticket_hash(account_id, ticket, &last_jar_index);
            let is_signature_valid = self.verify_signature(&signature.0, &pk, &hash);

            require!(is_signature_valid, "Not matching signature");

            let is_time_valid = env::block_timestamp_ms() <= ticket.valid_until.0;

            require!(is_time_valid, "Ticket is outdated");
        }
    }

    fn get_ticket_hash(
        &self,
        account_id: &AccountId,
        ticket: &JarTicket,
        last_jar_index: &JarIndex,
    ) -> Vec<u8> {
        sha256([
            env::current_account_id().as_bytes(),
            account_id.as_bytes(),
            ticket.product_id.as_bytes(),
            last_jar_index.to_string().as_bytes(),
            ticket.valid_until.0.to_string().as_bytes(),
        ].concat().as_slice())
    }

    fn verify_signature(
        &self,
        signature: &Vec<u8>,
        product_public_key: &Vec<u8>,
        ticket_hash: &Vec<u8>,
    ) -> bool {
        let signature = Signature::from_bytes(signature.as_slice())
            .expect("Invalid signature");

        PublicKey::from_bytes(product_public_key.as_slice())
            .expect("Public key is invalid")
            .verify_strict(ticket_hash.as_slice(), &signature)
            .is_ok()
    }
}