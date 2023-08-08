use std::cmp;

use near_sdk::{AccountId, env, near_bindgen, require};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::env::sha256;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use crate::common::{MINUTES_IN_YEAR, UDecimal};

use crate::*;
use crate::common::{MS_IN_MINUTE, Timestamp, TokenAmount};
use crate::event::{emit, EventKind};
use crate::product::{Apy, Product, ProductId};

pub type JarIndex = u32;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct JarTicket {
    pub product_id: String,
    pub valid_until: Timestamp,
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
pub struct JarView {
    pub index: JarIndex,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
    pub claimed_balance: U128,
    pub is_penalty_applied: bool,
}

impl From<Jar> for JarView {
    fn from(value: Jar) -> Self {
        Self {
            index: value.index,
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: U64(value.created_at),
            principal: U128(value.principal),
            claimed_balance: U128(value.claimed_balance),
            is_penalty_applied: value.is_penalty_applied,
        }
    }
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

pub trait JarApi {
    fn restake(&mut self, jar_index: JarIndex) -> JarView;

    fn get_jar(&self, jar_index: JarIndex) -> JarView;
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView>;

    fn get_total_principal(&self, account_id: AccountId) -> U128;
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> U128;

    fn get_total_interest(&self, account_id: AccountId) -> U128;
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> U128;
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
            let last_jar_index = self.account_jars.get(&account_id)
                .map_or_else(
                    || 0,
                    |jars| *jars.iter().max().unwrap(),
                );

            let hash = sha256([
                env::current_account_id().as_bytes(),
                account_id.as_bytes(),
                ticket.product_id.as_bytes(),
                last_jar_index.to_string().as_bytes(),
                ticket.valid_until.to_string().as_bytes(),
            ].concat().as_slice());

            let signature = Signature::from_bytes(signature.0.as_slice()).expect("Invalid signature");
            let is_signature_valid = PublicKey::from_bytes(pk.as_slice())
                .expect("Public key is invalid")
                .verify_strict(hash.as_slice(), &signature)
                .is_ok();

            require!(is_signature_valid, "Not matching signature");

            let is_time_valid = env::block_timestamp_ms() <= ticket.valid_until;

            require!(is_time_valid, "Ticket is outdated");
        }
    }
}

#[near_bindgen]
impl JarApi for Contract {
    fn restake(&mut self, jar_index: JarIndex) -> JarView {
        let jar = self.get_jar_internal(jar_index);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);

        let product = self.get_product(&jar.product_id);

        require!(product.is_restakable, "The product doesn't support restaking");

        let now = env::block_timestamp_ms();
        require!(jar.is_mature(&product, now), "The jar is not mature yet");

        let index = self.jars.len() as JarIndex;
        let new_jar = Jar::create(index, jar.account_id.clone(), jar.product_id.clone(), jar.principal, now);
        let withdraw_jar = jar.withdrawn(&product, jar.principal, now);

        self.save_jar(&account_id, &withdraw_jar);
        self.save_jar(&account_id, &new_jar);

        new_jar.into()
    }

    fn get_jar(&self, index: JarIndex) -> JarView {
        self.get_jar_internal(index).into()
    }

    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jar_ids(&account_id)
            .iter()
            .map(|index| self.get_jar(*index))
            .collect()
    }

    fn get_total_principal(&self, account_id: AccountId) -> U128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_principal(jar_indices)
    }

    // TODO: tests
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> U128 {
        let result = jar_indices
            .iter()
            .map(|index| self.get_jar_internal(*index).principal)
            .sum();

        U128(result)
    }

    fn get_total_interest(&self, account_id: AccountId) -> U128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_interest(jar_indices)
    }

    // TODO: tests
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> U128 {
        let now = env::block_timestamp_ms();
        let result = jar_indices
            .iter()
            .map(|index| self.get_jar_internal(*index))
            .map(|jar| jar.get_interest(&self.get_product(&jar.product_id), now))
            .sum();

        U128(result)
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
            0,
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
            0,
        );

        let interest = jar.get_interest(&product, 400 * 24 * 60 * 60 * 1000);
        assert_eq!(12_000_000, interest);
    }
}

#[cfg(test)]
mod signature_tests {
    use near_sdk::json_types::Base64VecU8;
    use near_sdk::test_utils::accounts;
    use crate::common::tests::Context;
    use crate::jar::JarTicket;
    use crate::product::{Product, ProductApi};
    use crate::product::tests::{get_premium_product, get_product};

    // Signature for structure (value -> utf8 bytes):
    // contract_id: "owner" -> [111, 119, 110, 101, 114]
    // account_id: "alice" -> [97, 108, 105, 99, 101]
    // product_id: "product_premium" -> [112, 114, 111, 100, 117, 99, 116, 95, 112, 114, 101, 109, 105, 117, 109]
    // last_jar_index: "0" -> [48]
    // valid_until: "100000000" -> [49, 48, 48, 48, 48, 48, 48, 48, 48]
    // ***
    // result array: [111, 119, 110, 101, 114, 97, 108, 105, 99, 101, 112, 114, 111, 100, 117, 99, 116, 95, 112, 114, 101, 109, 105, 117, 109, 48, 49, 48, 48, 48, 48, 48, 48, 48, 48]
    // sha256(result array): [215, 21, 45, 17, 130, 29, 202, 184, 32, 68, 245, 243, 252, 94, 251, 83, 166, 116, 97, 178, 137, 220, 227, 111, 162, 244, 203, 68, 178, 75, 140, 91]
    // ***
    // Secret: [87, 86, 114, 129, 25, 247, 248, 94, 16, 119, 169, 202, 195, 11, 187, 107, 195, 182, 205, 70, 189, 120, 214, 228, 208, 115, 234, 0, 244, 21, 218, 113]
    // Pk: [33, 80, 163, 149, 64, 30, 150, 45, 68, 212, 97, 122, 213, 118, 189, 174, 239, 109, 48, 82, 50, 35, 197, 176, 50, 211, 183, 128, 207, 1, 8, 68]
    // ***
    // SIGNATURE: [126, 76, 136, 40, 234, 193, 197, 143, 119, 86, 135, 170, 247, 130, 173, 154, 88, 43, 224, 78, 2, 2, 67, 243, 189, 28, 138, 43, 92, 93, 147, 187, 200, 62, 118, 158, 164, 108, 140, 154, 144, 147, 250, 112, 234, 255, 248, 213, 107, 224, 201, 147, 186, 233, 120, 56, 21, 160, 85, 204, 135, 240, 61, 13]
    fn get_valid_signature() -> Vec<u8> {
        vec![
            126, 76, 136, 40, 234, 193, 197, 143, 119, 86, 135, 170, 247, 130, 173, 154, 88, 43,
            224, 78, 2, 2, 67, 243, 189, 28, 138, 43, 92, 93, 147, 187, 200, 62, 118, 158, 164, 108,
            140, 154, 144, 147, 250, 112, 234, 255, 248, 213, 107, 224, 201, 147, 186, 233, 120, 56,
            21, 160, 85, 204, 135, 240, 61, 13,
        ]
    }

    #[test]
    fn verify_ticket_with_valid_signature_and_date() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_premium_product());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: 100000000,
        };

        context.contract.verify(&admin, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Invalid signature")]
    fn verify_ticket_with_invalid_signature() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_premium_product());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: 100000000,
        };

        let signature: Vec<u8> = vec![0, 1, 2];

        context.contract.verify(&admin, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Not matching signature")]
    fn verify_ticket_with_not_matching_signature() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);

        let product = Product {
            id: "another_product".to_string(),
            ..get_premium_product()
        };
        context.contract.register_product(product);

        let ticket = JarTicket {
            product_id: "another_product".to_string(),
            valid_until: 100000000,
        };

        let signature: Vec<u8> = [
            68, 119, 102, 0, 228, 169, 156, 208, 85, 165, 203, 130, 184, 28, 97, 129, 46, 72, 145,
            7, 129, 127, 17, 57, 153, 97, 38, 47, 101, 170, 168, 138, 44, 16, 162, 144, 53, 122,
            188, 128, 118, 102, 133, 165, 195, 42, 182, 22, 231, 99, 96, 72, 4, 79, 85, 143, 165,
            10, 200, 44, 160, 90, 120, 14
        ].to_vec();

        context.contract.verify(&admin, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Ticket is outdated")]
    fn verify_ticket_with_invalid_date() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.set_block_timestamp_in_days(365);
        context.contract.register_product(get_premium_product());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: 100000000,
        };

        context.contract.verify(&admin, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Product product_premium doesn't exist")]
    fn verify_ticket_with_not_existing_product() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: 100000000,
        };

        context.contract.verify(&admin, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Signature is required")]
    fn verify_ticket_without_signature_when_required() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_premium_product());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: 100000000,
        };

        context.contract.verify(&admin, &ticket, None);
    }

    #[test]
    fn verify_ticket_without_signature_when_not_required() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_product());

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: 0,
        };

        context.contract.verify(&admin, &ticket, None);
    }
}
