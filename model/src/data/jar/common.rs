use near_sdk::require;

use super::{Deposit, Jar, JarCache, JarCompanion};
use crate::{
    data::product::{Terms, TermsApi},
    Duration, Timestamp, TokenAmount,
};

pub trait Assertions {
    fn assert_not_locked(&self);
}

impl Assertions for Jar {
    fn assert_not_locked(&self) {
        require!(
            !self.is_pending_withdraw,
            "Another operation on this Jar is in progress"
        );
    }
}

impl Jar {
    pub fn total_principal(&self) -> TokenAmount {
        self.deposits.iter().map(|deposit| deposit.principal).sum()
    }

    pub fn get_liquid_balance(&self, terms: &Terms) -> (TokenAmount, usize) {
        if terms.allows_early_withdrawal() {
            let sum = self.deposits.iter().map(|deposit| deposit.principal).sum();
            let partition_index = self.deposits.len();

            (sum, partition_index)
        } else {
            let partition_index = self.deposits.partition_point(|deposit| terms.is_liquid(deposit));

            let sum = self.deposits[..partition_index]
                .iter()
                .map(|deposit| deposit.principal)
                .sum();

            (sum, partition_index)
        }
    }

    pub fn should_close(&self) -> bool {
        self.deposits.is_empty() && self.cache.is_none_or(|cache| cache.interest == 0)
    }

    pub fn lock(&mut self) -> &mut Self {
        self.is_pending_withdraw = true;

        self
    }

    pub fn try_lock(&mut self) -> &mut Self {
        self.assert_not_locked();
        self.lock()
    }

    pub fn unlock(&mut self) -> &mut Self {
        self.is_pending_withdraw = false;

        self
    }

    pub fn claim(&mut self, remainder: u64, now: Timestamp) -> &mut Self {
        self.claim_remainder = remainder;
        self.cache = Some(JarCache {
            updated_at: now,
            interest: 0,
        });

        self
    }

    pub fn update_cache(&mut self, interest: TokenAmount, remainder: u64, now: Timestamp) {
        self.cache = Some(JarCache {
            updated_at: now,
            interest,
        });
        self.claim_remainder = remainder;
    }

    pub fn clean_up_deposits(&mut self, partition_index: usize) {
        if partition_index == self.deposits.len() {
            self.deposits.clear();
        } else {
            self.deposits.drain(..partition_index);
        }
    }

    pub fn apply(&mut self, companion: &JarCompanion) -> &mut Self {
        if let Some(claim_remainder) = companion.claim_remainder {
            self.claim_remainder = claim_remainder;
        }

        if let Some(cache) = companion.cache {
            self.cache = cache;
        }

        if let Some(deposits) = &companion.deposits {
            self.deposits.clone_from(deposits);
        }

        if let Some(is_pending_withdraw) = companion.is_pending_withdraw {
            self.is_pending_withdraw = is_pending_withdraw;
        }

        self
    }

    pub fn to_rollback(&self) -> JarCompanion {
        JarCompanion {
            is_pending_withdraw: Some(false),
            claim_remainder: Some(self.claim_remainder),
            cache: Some(self.cache),
            ..JarCompanion::default()
        }
    }
}

impl Deposit {
    pub fn new(created_at: Timestamp, principal: TokenAmount) -> Self {
        Self { created_at, principal }
    }

    pub fn is_liquid(&self, now: Timestamp, term: Duration) -> bool {
        now - self.created_at > term
    }
}
