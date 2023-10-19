use std::cmp;

use ed25519_dalek::{VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use model::{jar::JarView, ProductId, TokenAmount};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    env::{panic_str, sha256},
    json_types::{U128, U64},
    require,
    serde::{Deserialize, Serialize},
    AccountId,
};

use crate::{
    common::{udecimal::UDecimal, Timestamp, MS_IN_YEAR},
    event::{emit, EventKind, TopUpData},
    product::model::{Apy, Product, Terms},
    Base64VecU8, Contract, JarsStorage, Signature,
};

pub type JarId = u32;

/// The `JarTicket` struct represents a request to create a deposit jar for a corresponding product.
///
/// The data from this `JarTicket` is later combined with additional data, including the contract
/// account address, the recipient's account ID, the desired amount of tokens to deposit,
/// and the ID of the last jar created for the recipient. The concatenation of this data
/// forms a message that is then hashed using the SHA-256 algorithm. This resulting hash is used
/// to verify the authenticity of the data against an Ed25519 signature provided in the `ft_transfer_call` data.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
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
#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
pub struct Jar {
    /// The unique identifier for the jar.
    pub id: JarId,

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

    /// Indicates whether a penalty has been applied to the jar's owner due to violating product terms.
    pub is_penalty_applied: bool,
}

/// A cached value that stores calculated interest based on the current state of the jar.
/// This cache is updated whenever properties that impact interest calculation change,
/// allowing for efficient interest calculations between state changes.
#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
#[serde(crate = "near_sdk::serde")]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: TokenAmount,
}

impl Jar {
    pub(crate) fn create(
        id: JarId,
        account_id: AccountId,
        product_id: ProductId,
        principal: TokenAmount,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            account_id,
            product_id,
            principal,
            created_at,
            cache: None,
            claimed_balance: 0,
            is_pending_withdraw: false,
            is_penalty_applied: false,
        }
    }

    pub(crate) fn lock(&mut self) {
        self.is_pending_withdraw = true;
    }

    pub(crate) fn unlocked(&self) -> Self {
        Self {
            is_pending_withdraw: false,
            ..self.clone()
        }
    }

    pub(crate) fn unlock(&mut self) {
        self.is_pending_withdraw = false;
    }

    pub(crate) fn apply_penalty(&mut self, is_applied: bool) {
        self.is_penalty_applied = is_applied;
    }

    pub(crate) fn top_up(&mut self, amount: TokenAmount, product: &Product, now: Timestamp) -> &mut Self {
        let current_interest = self.get_interest(product, now);

        self.principal += amount;
        self.cache = Some(JarCache {
            updated_at: now,
            interest: current_interest,
        });
        self
    }

    pub(crate) fn claim(
        &mut self,
        available_yield: TokenAmount,
        claimed_amount: TokenAmount,
        now: Timestamp,
    ) -> &mut Self {
        self.claimed_balance += claimed_amount;
        self.cache = Some(JarCache {
            updated_at: now,
            interest: available_yield - claimed_amount,
        });
        self
    }

    pub(crate) fn withdrawn(&self, product: &Product, withdrawn_amount: TokenAmount, now: Timestamp) -> Self {
        Self {
            principal: self.principal - withdrawn_amount,
            cache: Some(JarCache {
                updated_at: now,
                interest: self.get_interest(product, now),
            }),
            ..self.clone()
        }
    }

    pub(crate) fn should_be_closed(&self, product: &Product, now: Timestamp) -> bool {
        !product.is_flexible() && self.principal == 0 && self.get_interest(product, now) == 0
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

    pub(crate) fn is_empty(&self) -> bool {
        self.principal == 0
    }

    pub(crate) fn get_interest(&self, product: &Product, now: Timestamp) -> TokenAmount {
        let (base_date, base_interest) = if let Some(cache) = &self.cache {
            (cache.updated_at, cache.interest)
        } else {
            (self.created_at, 0)
        };
        let until_date = self.get_interest_until_date(product, now);
        let effective_term = if until_date > base_date {
            until_date - base_date
        } else {
            0
        };

        let term_in_minutes = u128::from(effective_term);
        let apy = self.get_apy(product);
        let total_interest = apy * self.principal;

        let interest = (term_in_minutes * total_interest) / u128::from(MS_IN_YEAR);

        base_interest + interest
    }

    fn get_apy(&self, product: &Product) -> UDecimal {
        match product.apy.clone() {
            Apy::Constant(apy) => apy,
            Apy::Downgradable(apy) => {
                if self.is_penalty_applied {
                    apy.fallback
                } else {
                    apy.default
                }
            }
        }
    }

    fn get_interest_until_date(&self, product: &Product, now: Timestamp) -> Timestamp {
        match product.terms.clone() {
            Terms::Fixed(value) => cmp::min(now, self.created_at + value.lockup_term),
            Terms::Flexible => now,
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
        let product_id = &ticket.product_id;
        let product = self.get_product(product_id);

        product.assert_enabled();
        product.assert_cap(amount);
        self.verify(&account_id, amount, &ticket, signature);

        let id = self.increment_and_get_last_jar_id();
        let now = env::block_timestamp_ms();
        let jar = Jar::create(id, account_id.clone(), product_id.clone(), amount, now);

        self.add_new_jar(&account_id, jar.clone());

        emit(EventKind::CreateJar(jar.clone()));

        jar.into()
    }

    pub(crate) fn top_up(&mut self, account: &AccountId, jar_id: JarId, amount: U128) -> U128 {
        let jar = self.get_jar_internal(account, jar_id);
        let product = self.get_product(&jar.product_id).clone();

        require!(product.allows_top_up(), "The product doesn't allow top-ups");
        product.assert_cap(jar.principal + amount.0);

        let now = env::block_timestamp_ms();

        let principal = self
            .get_jar_mut_internal(account, jar_id)
            .top_up(amount.0, &product, now)
            .principal;

        emit(EventKind::TopUp(TopUpData { id: jar_id, amount }));

        U128(principal)
    }

    pub(crate) fn delete_jar(&mut self, account_id: &AccountId, jar_id: JarId) {
        let jars = self
            .account_jars
            .get_mut(account_id)
            .unwrap_or_else(|| panic_str(&format!("Account '{account_id}' doesn't exist")));

        require!(
            !jars.is_empty(),
            "Trying to delete a jar from account without any jars."
        );

        let jar_position = jars
            .iter()
            .position(|j| j.id == jar_id)
            .unwrap_or_else(|| panic_str(&format!("Jar with id {jar_id} doesn't exist")));

        jars.swap_remove(jar_position);
    }

    pub(crate) fn get_jar_mut_internal(&mut self, account: &AccountId, id: JarId) -> &mut Jar {
        self.account_jars
            .get_mut(account)
            .unwrap_or_else(|| panic_str(&format!("Account '{account}' doesn't exist")))
            .get_jar_mut(id)
    }

    pub(crate) fn get_jar_internal(&self, account: &AccountId, id: JarId) -> &Jar {
        self.account_jars
            .get(account)
            .unwrap_or_else(|| env::panic_str(&format!("Account '{account}' doesn't exist")))
            .get_jar(id)
    }

    pub(crate) fn verify(
        &mut self,
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &JarTicket,
        signature: Option<Base64VecU8>,
    ) {
        let last_jar_id = self.account_jars.get(account_id).map(|jars| jars.last_id);
        let product = self.get_product(&ticket.product_id);

        if let Some(pk) = &product.public_key {
            let Some(signature) = signature else {
                panic_str("Signature is required");
            };

            let is_time_valid = env::block_timestamp_ms() <= ticket.valid_until.0;
            require!(is_time_valid, "Ticket is outdated");

            let hash = Self::get_ticket_hash(account_id, amount, ticket, last_jar_id);
            let is_signature_valid = Self::verify_signature(&signature.0, pk, &hash);

            require!(is_signature_valid, "Not matching signature");
        }
    }

    fn get_ticket_hash(
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &JarTicket,
        last_jar_id: Option<JarId>,
    ) -> Vec<u8> {
        sha256(
            Self::get_signature_material(
                &env::current_account_id(),
                account_id,
                &ticket.product_id,
                amount,
                ticket.valid_until.0,
                last_jar_id,
            )
            .as_bytes(),
        )
    }

    pub(crate) fn get_signature_material(
        contract_account_id: &AccountId,
        receiver_account_id: &AccountId,
        product_id: &ProductId,
        amount: TokenAmount,
        valid_until: Timestamp,
        last_jar_id: Option<JarId>,
    ) -> String {
        format!(
            "{},{},{},{},{},{}",
            contract_account_id,
            receiver_account_id,
            product_id,
            amount,
            last_jar_id.map_or_else(String::new, |value| value.to_string(),),
            valid_until,
        )
    }

    fn verify_signature(signature: &[u8], product_public_key: &[u8], ticket_hash: &[u8]) -> bool {
        let signature_bytes: &[u8; SIGNATURE_LENGTH] = signature
            .try_into()
            .unwrap_or_else(|_| panic!("Signature must be {SIGNATURE_LENGTH} bytes"));

        let signature = Signature::from_bytes(signature_bytes);

        let public_key_bytes: &[u8; PUBLIC_KEY_LENGTH] = product_public_key
            .try_into()
            .unwrap_or_else(|_| panic!("Public key must be {PUBLIC_KEY_LENGTH} bytes"));

        VerifyingKey::from_bytes(public_key_bytes)
            .expect("Public key is invalid")
            .verify_strict(ticket_hash, &signature)
            .is_ok()
    }
}
