use std::cmp;

use near_sdk::{AccountId, env, require};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::env::sha256;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::common::{MINUTES_IN_YEAR, UDecimal};
use crate::common::{MS_IN_MINUTE, Timestamp, TokenAmount};
use crate::event::{emit, EventKind, TopUpData};
use crate::jar::view::JarView;
use crate::product::model::{Apy, Product, ProductId, Terms};

pub type JarIndex = u32;

/// The `JarTicket` struct represents a request to create a deposit jar for a corresponding product.
///
/// The data from this JarTicket is later combined with additional data, including the contract
/// account address, the recipient's account ID, the desired amount of tokens to deposit,
/// and the ID of the last jar created for the recipient. The concatenation of this data
/// forms a message that is then hashed using the SHA-256 algorithm. This resulting hash is used
/// to verify the authenticity of the data against an Ed25519 signature provided in the ft_transfer_call data.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct JarTicket {
    /// The unique identifier of the product for which the jar is intended to be created.
    /// This product_id links the request to the specific terms and conditions of the product that will govern the behavior of the jar.
    pub product_id: String,

    /// Specifies the expiration date of the ticket. The expiration date is measured in milliseconds
    /// since the Unix epoch. This property ensures that the request to create a jar is valid only
    /// until the specified timestamp. After this timestamp, the ticket becomes
    /// invalid and should not be accepted.
    pub valid_until: U64,
}

/// The `Jar` struct represents a deposit jar within the smart contract.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct Jar {
    /// The index of the jar in the `Contracts.jars` vector. Also serves as the unique identifier for the jar.
    pub index: JarIndex,

    /// The account ID of the owner of the jar.
    pub account_id: AccountId,

    /// The product ID that describes the terms of the deposit associated with the jar.
    pub product_id: ProductId,

    /// The timestamp of when the jar was created, measured in milliseconds since Unix epoch.
    pub created_at: Timestamp,

    /// The principal amount of the deposit stored in the jar.
    pub principal: TokenAmount,

    /// A cached value that stores calculated interest based on the current state of the jar.
    /// This cache is updated whenever properties that impact interest calculation change,
    /// allowing for efficient interest calculations between state changes.
    pub cache: Option<JarCache>,

    /// The amount of tokens that have been claimed from the jar up to the present moment.
    pub claimed_balance: TokenAmount,

    /// Indicates whether an operation involving cross-contract calls is in progress for this jar.
    pub is_pending_withdraw: bool,

    /// The state of the jar, indicating whether it is active or closed.
    pub state: JarState,

    /// Indicates whether a penalty has been applied to the jar's owner due to violating product terms.
    pub is_penalty_applied: bool,
}

/// A cached value that stores calculated interest based on the current state of the jar.
/// This cache is updated whenever properties that impact interest calculation change,
/// allowing for efficient interest calculations between state changes.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: TokenAmount,
}

/// The state of a jar, indicating whether it is active or closed.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
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

    /// Indicates whether a user can withdraw tokens from the jar at the moment or not.
    /// For a Flexible product withdrawal is always possible.
    /// For Fixed product it's defined by the lockup term.
    pub(crate) fn is_liquidable(&self, product: &Product, now: Timestamp) -> bool {
        match product.clone().terms {
            Terms::Fixed(value) => now - self.created_at > value.lockup_term,
            Terms::Flexible => true,
        }
    }

    pub(crate) fn get_interest(&self, product: &Product, now: Timestamp) -> TokenAmount {
        let (base_date, base_interest) = if let Some(cache) = &self.cache {
            (cache.updated_at, cache.interest)
        } else {
            (self.created_at, 0)
        };
        let until_date = self.get_interest_until_date(product, now);

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

    fn get_interest_until_date(&self, product: &Product, now: Timestamp) -> Timestamp {
        match product.terms.clone() {
            Terms::Fixed(value) => cmp::min(now, self.created_at + value.lockup_term),
            Terms::Flexible => now
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

        product.assert_enabled();
        product.assert_cap(amount);
        self.verify(&account_id, 1_000_000, &ticket, signature);

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

        require!(product.allows_top_up(), "The product doesn't allow top-ups");
        product.assert_cap(jar.principal + amount.0);

        let now = env::block_timestamp_ms();
        let topped_up_jar = jar.topped_up(amount.0, &product, now);

        self.jars.replace(jar_index, topped_up_jar.clone());

        emit(EventKind::TopUp(TopUpData { index: jar_index, amount }));

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

    pub(crate) fn verify(
        &self,
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &JarTicket,
        signature: Option<Base64VecU8>,
    ) {
        let product = self.get_product(&ticket.product_id);
        if let Some(pk) = product.public_key {
            let signature = signature.expect("Signature is required");
            let last_jar_index = self.account_jars.get(account_id)
                .map_or_else(
                    || 0,
                    |jars| *jars.iter().max().unwrap(),
                );

            let hash = self.get_ticket_hash(account_id, amount, ticket, &last_jar_index);
            let is_signature_valid = self.verify_signature(&signature.0, &pk, &hash);

            require!(is_signature_valid, "Not matching signature");

            let is_time_valid = env::block_timestamp_ms() <= ticket.valid_until.0;

            require!(is_time_valid, "Ticket is outdated");
        }
    }

    fn get_ticket_hash(
        &self,
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &JarTicket,
        last_jar_index: &JarIndex,
    ) -> Vec<u8> {
        sha256(
            format!(
                "{},{},{},{},{},{}",
                env::current_account_id(),
                account_id,
                ticket.product_id,
                amount,
                last_jar_index,
                ticket.valid_until.0
            ).as_bytes()
        )
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