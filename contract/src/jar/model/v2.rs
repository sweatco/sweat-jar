use near_sdk::{near, AccountId};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount, UDecimal, MS_IN_YEAR};

use crate::{common::Timestamp, jar::model::JarCache};

/// The `Jar` struct represents a deposit jar within the smart contract.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarV2 {
    pub deposits: Vec<Deposit>,
    pub cache: Option<JarCache>,
    pub claimed_balance: TokenAmount,
    pub is_pending_withdraw: bool,
    pub claim_remainder: u64,
}

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Deposit {
    pub created_at: Timestamp,
    pub principal: TokenAmount,
}

impl Deposit {
    fn get_interest_with_apy(
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

    fn get_interest_for_term(&self, apy: UDecimal, term: Timestamp) -> (TokenAmount, u64) {
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

    fn is_liquidable(&self, now: Timestamp, term: Duration) -> bool {
        now - self.created_at > term
    }
}
