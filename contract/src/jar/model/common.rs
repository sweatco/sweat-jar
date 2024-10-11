use std::cmp;

use near_sdk::{
    env,
    json_types::{Base64VecU8, U128, U64},
    near, AccountId,
};
use sweat_jar_model::{jar::JarId, Score, Timezone, TokenAmount, UDecimal, MS_IN_DAY, MS_IN_YEAR};

use crate::{
    common::Timestamp,
    jar::model::{Jar, JarLastVersion},
    product::model::{
        v1::{Apy, Product},
        v2::{ProductV2, Terms},
    },
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
    pub(crate) fn deposit(
        &mut self,
        account_id: AccountId,
        ticket: JarTicket,
        amount: U128,
        signature: Option<Base64VecU8>,
    ) {
        let amount = amount.0;
        let product_id = &ticket.product_id;
        let product = self.get_product(product_id);

        product.assert_enabled();
        product.assert_cap(amount);
        self.verify(&account_id, amount, &ticket, signature);

        let account = self.get_or_create_account_mut(&account_id);

        if matches!(product.terms, Terms::ScoreBased(_)) {
            account.try_set_timezone(ticket.timezone);
        }

        let account = self.get_or_create_account_mut(&account_id);
        account.deposit(product_id, amount);
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
}

impl JarLastVersion {
    pub(crate) fn get_interest(&self, score: &[Score], product: &ProductV2, now: Timestamp) -> (TokenAmount, u64) {
        if product.is_score_product() {
            self.get_score_interest(score, product, now)
        } else {
            self.get_interest_with_apy(self.get_apy(product), product, now)
        }
    }

    fn get_apy(&self, product: &ProductV2) -> UDecimal {
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

    fn get_interest_with_apy(&self, apy: UDecimal, product: &ProductV2, now: Timestamp) -> (TokenAmount, u64) {
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

    fn get_score_interest(&self, score: &[Score], product: &ProductV2, now: Timestamp) -> (TokenAmount, u64) {
        let cache = self.cache.map(|c| c.interest).unwrap_or_default();

        if let Terms::Fixed(end_term) = &product.terms {
            if now > end_term.lockup_term {
                return (cache, 0);
            }
        }

        let apy = product.apy_for_score(score);
        self.get_interest_for_term(cache, apy, MS_IN_DAY)
    }

    fn get_interest_until_date(&self, product: &ProductV2, now: Timestamp) -> Timestamp {
        match product.terms.clone() {
            Terms::Fixed(value) => cmp::min(now, self.created_at + value.lockup_term),
            Terms::ScoreBased(value) => cmp::min(now, self.created_at + value.lockup_term),
            Terms::Flexible(_) => now,
        }
    }
}
