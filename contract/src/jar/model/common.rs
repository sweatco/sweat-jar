use std::cmp;

use ed25519_dalek::{Signature, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use near_sdk::{
    env,
    env::{panic_str, sha256},
    json_types::{Base64VecU8, U128, U64},
    near, require, AccountId,
};
use sweat_jar_model::{
    jar::{JarId, JarView},
    ProductId, ScoreRecord, Timezone, TokenAmount, UDecimal, MS_IN_DAY, MS_IN_YEAR,
};

use crate::{
    common::Timestamp,
    event::{emit, EventKind, TopUpData},
    jar::model::{Jar, JarLastVersion},
    product::model::{Apy, Product, Terms},
    score::AccountScore,
    Contract, JarsStorage,
};

/// The `JarTicket` struct represents a request to create a deposit jar for a corresponding product.
///
/// The data from this `JarTicket` is later combined with additional data, including the contract
/// account address, the recipient's account ID, the desired amount of tokens to deposit,
/// and the ID of the last jar created for the recipient. The concatenation of this data
/// forms a message that is then hashed using the SHA-256 algorithm. This resulting hash is used
/// to verify the authenticity of the data against an Ed25519 signature provided in the `ft_transfer_call` data.
#[derive(Clone, Debug)]
#[near(serializers=[json])]
pub struct JarTicket {
    /// The unique identifier of the product for which the jar is intended to be created.
    /// This `product_id` links the request to the specific terms and conditions of the product that will govern the behavior of the jar.
    pub product_id: String,

    /// Specifies the expiration date of the ticket. The expiration date is measured in milliseconds
    /// since the Unix epoch. This property ensures that the request to create a jar is valid only
    /// until the specified timestamp. After this timestamp, the ticket becomes
    /// invalid and should not be accepted.
    pub valid_until: U64,

    /// An optional user timezone. Required for creating step jars.
    pub timezone: Option<Timezone>,
}

impl JarLastVersion {
    pub(crate) fn lock(&mut self) {
        self.is_pending_withdraw = true;
    }

    pub(crate) fn unlock(&mut self) {
        self.is_pending_withdraw = false;
    }

    pub(crate) fn apply_penalty(&mut self, product: &Product, is_applied: bool, now: Timestamp) {
        assert!(
            !product.is_score_product(),
            "Applying penalty is not supported for score based jars"
        );

        let (interest, remainder) = self.get_interest(&ScoreRecord::default(), product, now);

        self.claim_remainder = remainder;

        self.cache = Some(JarCache {
            updated_at: now,
            interest,
        });
        self.is_penalty_applied = is_applied;
    }

    pub(crate) fn top_up(&mut self, amount: TokenAmount, product: &Product, now: Timestamp) -> &mut Self {
        assert!(
            !product.is_score_product(),
            "Top up is not supported for score based jars"
        );

        let current_interest = self.get_interest(&ScoreRecord::default(), product, now).0;

        self.principal += amount;
        self.cache = Some(JarCache {
            updated_at: now,
            interest: current_interest,
        });
        self
    }

    pub(crate) fn claim(&mut self, claimed_amount: TokenAmount, now: Timestamp) -> &mut Self {
        self.claimed_balance += claimed_amount;

        self.cache = Some(JarCache {
            updated_at: now,
            interest: 0,
        });
        self
    }

    pub(crate) fn should_be_closed(&self, score: &ScoreRecord, product: &Product, now: Timestamp) -> bool {
        !product.is_flexible() && self.principal == 0 && self.get_interest(score, product, now).0 == 0
    }

    /// Indicates whether a user can withdraw tokens from the jar at the moment or not.
    /// For a Flexible product withdrawal is always possible.
    /// For Fixed product it's defined by the lockup term.
    pub(crate) fn is_liquidable(&self, product: &Product, now: Timestamp) -> bool {
        match &product.terms {
            Terms::Fixed(value) => now - self.created_at > value.lockup_term,
            Terms::Flexible => true,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.principal == 0
    }

    fn get_interest_for_term(&self, cache: u128, apy: UDecimal, term: Timestamp) -> (TokenAmount, u64) {
        let term_in_milliseconds: u128 = term.into();

        let yearly_interest = apy * self.principal;

        let ms_in_year: u128 = MS_IN_YEAR.into();

        let interest = term_in_milliseconds * yearly_interest;

        // This will never fail because `MS_IN_YEAR` is u64
        // and remainder from u64 cannot be bigger than u64 so it is safe to unwrap here.
        let remainder: u64 = (interest % ms_in_year).try_into().unwrap();

        let interest = interest / ms_in_year;

        let total_remainder = self.claim_remainder + remainder;

        (
            cache + interest + u128::from(total_remainder / MS_IN_YEAR),
            total_remainder % MS_IN_YEAR,
        )
    }

    fn get_interest_with_apy(&self, apy: UDecimal, product: &Product, now: Timestamp) -> (TokenAmount, u64) {
        let (base_date, cache_interest) = if let Some(cache) = &self.cache {
            (cache.updated_at, cache.interest)
        } else {
            (self.created_at, 0)
        };

        let until_date = self.get_interest_until_date(product, now);

        let effective_term = if until_date > base_date {
            until_date - base_date
        } else {
            return (cache_interest, 0);
        };

        self.get_interest_for_term(cache_interest, apy, effective_term)
    }

    fn get_score_interest(&self, score: &ScoreRecord, product: &Product, now: Timestamp) -> (TokenAmount, u64) {
        let cache = self.cache.map(|c| c.interest).unwrap_or_default();

        // The score was updated before jars creation
        if score.updated.0 < self.created_at {
            return (cache, 0);
        }

        if let Terms::Fixed(end_term) = &product.terms {
            let end_term = cmp::max(now, self.created_at + end_term.lockup_term);
            if now >= end_term {
                return (cache, 0);
            }
        }

        let apy = product.apy_for_score(&score.score);
        self.get_interest_for_term(cache, apy, MS_IN_DAY)
    }

    pub(crate) fn get_interest(&self, score: &ScoreRecord, product: &Product, now: Timestamp) -> (TokenAmount, u64) {
        if product.is_score_product() {
            self.get_score_interest(score, product, now)
        } else {
            self.get_interest_with_apy(self.get_apy(product), product, now)
        }
    }

    pub(crate) fn get_apy(&self, product: &Product) -> UDecimal {
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

/// A cached value that stores calculated interest based on the current state of the jar.
/// This cache is updated whenever properties that impact interest calculation change,
/// allowing for efficient interest calculations between state changes.
#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: TokenAmount,
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

        self.migrate_account_if_needed(&account_id);

        self.verify(&account_id, amount, &ticket, signature);

        if product.is_score_product() {
            match (ticket.timezone, self.get_score_mut(&account_id)) {
                // Time zone already set. No actions required.
                (Some(_) | None, Some(_)) => (),
                (Some(timezone), None) => {
                    self.accounts.entry(account_id.clone()).or_default().score = AccountScore::new(timezone);
                }
                (None, None) => {
                    panic_str(&format!(
                        "Trying to create step base jar for account: '{account_id}' without providing time zone"
                    ));
                }
            }
        }

        let id = self.increment_and_get_last_jar_id();
        let now = env::block_timestamp_ms();
        let jar = Jar::create(id, account_id.clone(), product_id.clone(), amount, now);

        self.add_new_jar(&account_id, jar.clone());

        emit(EventKind::CreateJar(jar.clone().into()));

        jar.into()
    }

    pub(crate) fn top_up(&mut self, account: &AccountId, jar_id: JarId, amount: U128) -> U128 {
        self.migrate_account_if_needed(account);

        let jar = self.get_jar_internal(account, jar_id).clone();
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
            .accounts
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

    pub(crate) fn get_score(&self, account: &AccountId) -> Option<&AccountScore> {
        self.accounts.get(account).and_then(|a| a.score())
    }

    pub(crate) fn get_score_mut(&mut self, account: &AccountId) -> Option<&mut AccountScore> {
        self.accounts.get_mut(account).and_then(|a| a.score_mut())
    }

    pub(crate) fn get_jar_mut_internal(&mut self, account: &AccountId, id: JarId) -> &mut Jar {
        self.accounts
            .get_mut(account)
            .unwrap_or_else(|| env::panic_str(&format!("Account '{account}' doesn't exist")))
            .get_jar_mut(id)
    }

    #[mutants::skip]
    pub(crate) fn get_jar_internal(&self, account: &AccountId, id: JarId) -> Jar {
        if let Some(jars) = self.account_jars_v1.get(account) {
            return jars
                .jars
                .iter()
                .find(|jar| jar.id == id)
                .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {id} doesn't exist")))
                .clone()
                .into();
        }

        if let Some(jars) = self.account_jars_non_versioned.get(account) {
            return jars
                .jars
                .iter()
                .find(|jar| jar.id == id)
                .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {id} doesn't exist")))
                .clone();
        }

        self.accounts
            .get(account)
            .unwrap_or_else(|| env::panic_str(&format!("Account '{account}' doesn't exist")))
            .get_jar(id)
            .clone()
    }

    pub(crate) fn verify(
        &mut self,
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &JarTicket,
        signature: Option<Base64VecU8>,
    ) {
        self.migrate_account_if_needed(account_id);

        let last_jar_id = self.accounts.get(account_id).map(|jars| jars.last_id);
        let product = self.get_product(&ticket.product_id);

        if let Some(pk) = &product.public_key {
            let Some(signature) = signature else {
                panic_str("Signature is required");
            };

            let is_time_valid = env::block_timestamp_ms() <= ticket.valid_until.0;
            require!(is_time_valid, "Ticket is outdated");

            let signature_material = Self::get_signature_material(
                &env::current_account_id(),
                account_id,
                &ticket.product_id,
                amount,
                last_jar_id,
                ticket.valid_until.0,
            );

            let hash = Self::get_ticket_hash(&signature_material);
            let is_signature_valid = Self::verify_signature(&signature.0, pk, &hash);

            if !is_signature_valid {
                panic_str(&format!(
                    "Not matching signature. Signature material: {signature_material}"
                ));
            }
        }
    }

    fn get_ticket_hash(signature_material: &str) -> Vec<u8> {
        sha256(signature_material.as_bytes())
    }

    pub(crate) fn get_signature_material(
        contract_account_id: &AccountId,
        receiver_account_id: &AccountId,
        product_id: &ProductId,
        amount: TokenAmount,
        last_jar_id: Option<JarId>,
        valid_until: Timestamp,
    ) -> String {
        format!(
            "{},{},{},{},{},{}",
            contract_account_id,
            receiver_account_id,
            product_id,
            amount,
            last_jar_id.map_or_else(String::new, |value| value.to_string()),
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
