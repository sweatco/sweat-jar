use std::cmp;

use near_sdk::{
    json_types::U64,
    near,
    serde::{Deserialize, Serialize},
};
use sweat_jar_model::{TokenAmount, MS_IN_YEAR};

use crate::{
    common::{udecimal::UDecimal, Duration, Timestamp},
    product::model::{Product, Terms},
};

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

/// A cached value that stores calculated interest based on the current state of the jar.
/// This cache is updated whenever properties that impact interest calculation change,
/// allowing for efficient interest calculations between state changes.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: TokenAmount,
}

/// TODO: add doc
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Deposit {
    /// The timestamp of when the jar was created, measured in milliseconds since Unix epoch.
    pub created_at: Timestamp,

    /// The principal amount of the deposit stored in the jar.
    pub principal: TokenAmount,
}

impl Deposit {
    pub fn new(created_at: Timestamp, principal: TokenAmount) -> Self {
        Deposit { created_at, principal }
    }
}

impl Deposit {
    pub(crate) fn get_interest_with_apy(
        &self,
        apy: UDecimal,
        product: &Product,
        now: Timestamp,
        since_date: Option<Timestamp>,
    ) -> (TokenAmount, u64) {
        let since_date = since_date.unwrap_or(self.created_at);

        let until_date = self.get_interest_until_date(product, now);

        let effective_term = if until_date > since_date {
            until_date - since_date
        } else {
            return (0, 0);
        };

        self.get_interest_for_term(apy, effective_term)
    }

    pub(crate) fn get_interest_for_term(&self, apy: UDecimal, term: Timestamp) -> (TokenAmount, u64) {
        let term_in_milliseconds: u128 = term.into();

        let yearly_interest = apy * self.principal;

        let ms_in_year: u128 = MS_IN_YEAR.into();

        let interest = term_in_milliseconds * yearly_interest;

        // This will never fail because `MS_IN_YEAR` is u64
        // and remainder from u64 cannot be bigger than u64 so it is safe to unwrap here.
        let remainder: u64 = (interest % ms_in_year).try_into().unwrap();
        let interest = interest / ms_in_year;

        (interest, remainder)
    }

    fn get_interest_until_date(&self, product: &Product, now: Timestamp) -> Timestamp {
        match product.terms.clone() {
            Terms::Fixed(value) => cmp::min(now, self.created_at + value.lockup_term),
            Terms::Flexible => now,
        }
    }

    pub(crate) fn is_liquidable(&self, now: Timestamp, term: Duration) -> bool {
        now - self.created_at > term
    }
}
