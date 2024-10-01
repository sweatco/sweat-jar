use std::cmp;

use near_sdk::{near, AccountId};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount, MS_IN_DAY, MS_IN_YEAR};

use crate::{
    common::{udecimal::UDecimal, Timestamp},
    jar::model::common::{Deposit, JarCache},
    product::model::{Apy, Product, Terms},
};

/// The `Jar` struct represents a deposit jar within the smart contract.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub struct JarV2 {
    /// The unique identifier for the jar.
    pub id: JarId,

    /// The account ID of the owner of the jar.
    pub account_id: AccountId,

    /// The product ID that describes the terms of the deposit associated with the jar.
    pub product_id: ProductId,

    /// TODO: add doc
    pub deposits: Vec<Deposit>,

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

    /// Remainder of claim operation.
    /// Needed to negate rounding error when user claims very often.
    /// See `Jar::get_interest` method for implementation of this logic.
    pub claim_remainder: u64,
}

impl JarV2 {
    pub(crate) fn lock(&mut self) {
        self.is_pending_withdraw = true;
    }

    pub(crate) fn unlock(&mut self) {
        self.is_pending_withdraw = false;
    }

    pub(crate) fn principal(&self) -> TokenAmount {
        self.deposits.iter().map(|deposit| deposit.principal).sum()
    }

    pub(crate) fn liquidable_principal(&self, product: &Product, now: Timestamp) -> TokenAmount {
        match &product.terms {
            Terms::Fixed(value) => self
                .deposits
                .iter()
                .filter_map(|deposit: Deposit| {
                    if deposit.is_liquidable(now, value.lockup_term) {
                        Some(deposit.principal)
                    } else {
                        None
                    }
                })
                .sum(),
            Terms::Flexible => {
                self.deposits
                    .first()
                    .expect("Flexible product must contain single deposit")
                    .principal
            }
        }
    }

    pub(crate) fn withdraw(&mut self, score: &[Score], product: &Product, now: Timestamp) -> TokenAmount {
        let (interest, interest_remainder) = self.get_interest(score, product, now);
        self.cache = Some(JarCache {
            updated_at: now,
            interest,
        });
        self.claim_remainder += interest_remainder;

        if !self.is_liquidable(product, now) {
            return 0;
        }

        match &product.terms {
            Terms::Fixed(product_terms) => {
                let mut last_mature_deposit_index = 0;
                let mut amount_to_withdraw = 0;

                for (index, deposit) in self.deposits.iter().enumerate() {
                    if !deposit.is_liquidable(now, product_terms.lockup_term) {
                        break;
                    }

                    last_mature_deposit_index = index;
                    amount_to_withdraw += deposit.principal;
                }

                self.deposits.drain(0..last_mature_deposit_index);

                amount_to_withdraw
            }
            Terms::Flexible => {
                self.deposits
                    .first()
                    .expect("Flexible product must contain single deposit")
                    .principal
            }
        }
    }

    pub(crate) fn deposit(&mut self, amount: TokenAmount, now: Timestamp) {
        self.deposits.push(Deposit::new(now, amount));
    }

    pub(crate) fn apply_penalty(&mut self, product: &Product, is_applied: bool, now: Timestamp) {
        assert!(
            !product.is_score_product(),
            "Applying penalty is not supported for score based jars"
        );

        let current_interest = self.get_interest(&[], product, now).0;

        self.cache = Some(JarCache {
            updated_at: now,
            interest: current_interest,
        });
        self.is_penalty_applied = is_applied;
    }

    pub(crate) fn top_up(&mut self, amount: TokenAmount, product: &Product, now: Timestamp) -> &mut Self {
        assert!(
            !product.is_score_product(),
            "Top up is not supported for score based jars"
        );

        let current_interest = self.get_interest(&[], product, now).0;

        self.deposits.first_mut().expect("Deposits are empty").principal += amount;
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

    pub(crate) fn should_be_closed(&self, score: &[Score], product: &Product, now: Timestamp) -> bool {
        !product.is_flexible() && self.principal() == 0 && self.get_interest(score, product, now).0 == 0
    }

    /// Indicates whether a user can withdraw tokens from the jar at the moment or not.
    /// For a Flexible product withdrawal is always possible.
    /// For Fixed product it's defined by the lockup term.
    pub(crate) fn is_liquidable(&self, product: &Product, now: Timestamp) -> bool {
        match &product.terms {
            Terms::Fixed(product_terms) => self.deposits.first().map_or_else(
                |deposit: &Deposit| deposit.is_liquidable(now, product_terms.lockup_term),
                false,
            ),
            Terms::Flexible => true,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.principal() == 0
    }

    pub(crate) fn get_interest(&self, score: &[Score], product: &Product, now: Timestamp) -> (TokenAmount, u64) {
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

    fn get_interest_with_apy(&self, apy: UDecimal, product: &Product, now: Timestamp) -> (TokenAmount, u64) {
        let (cache_date, cached_interest) = if let Some(cache) = &self.cache {
            (Some(cache.updated_at), cache.interest)
        } else {
            (None, 0)
        };

        let mut interest = 0;
        let mut remainder = 0;
        for deposit in self.deposits.iter() {
            let deposit_interest = deposit.get_interest_with_apy(apy, product, now, cache_date);
            interest += deposit_interest.0;
            remainder += deposit_interest.1;
        }

        (
            cached_interest + interest + u128::from(remainder / MS_IN_YEAR),
            remainder % MS_IN_YEAR,
        )
    }

    fn get_score_interest(&self, score: &[Score], product: &Product, now: Timestamp) -> (TokenAmount, u64) {
        let cached_interest = self.cache.map(|c| c.interest).unwrap_or_default();

        if let Terms::Fixed(end_term) = &product.terms {
            if now > end_term.lockup_term {
                return (cached_interest, 0);
            }
        }

        let apy = product.apy_for_score(score);

        let mut interest = 0;
        let mut remainder = 0;
        for deposit in self.deposits.iter() {
            let deposit_interest = deposit.get_interest_for_term(apy, MS_IN_DAY);
            interest += deposit_interest.0;
            remainder += deposit_interest.1;
        }

        (
            cached_interest + interest + u128::from(remainder / MS_IN_YEAR),
            remainder % MS_IN_YEAR,
        )
    }
}
