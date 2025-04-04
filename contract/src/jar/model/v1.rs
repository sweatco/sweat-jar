use near_sdk::near;
use sweat_jar_model::{product::Terms, TokenAmount};

use crate::{
    assert::assert_not_locked,
    common::{Duration, Timestamp},
    jar::model::JarCache,
    product::model::v1::TermsApi,
};

/// The `Jar` struct represents a deposit jar within the smart contract.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct Jar {
    pub deposits: Vec<Deposit>,
    pub cache: Option<JarCache>,
    pub is_pending_withdraw: bool,
    pub claim_remainder: u64,
}

#[allow(clippy::option_option)]
#[near(serializers=[json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct JarCompanion {
    pub deposits: Option<Vec<Deposit>>,
    pub cache: Option<Option<JarCache>>,
    pub is_pending_withdraw: Option<bool>,
    pub claim_remainder: Option<u64>,
}

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Deposit {
    pub created_at: Timestamp,
    pub principal: TokenAmount,
}

impl Deposit {
    pub(crate) fn new(created_at: Timestamp, principal: TokenAmount) -> Self {
        Self { created_at, principal }
    }

    pub(crate) fn is_liquid(&self, now: Timestamp, term: Duration) -> bool {
        now - self.created_at > term
    }
}

impl Jar {
    pub(crate) fn total_principal(&self) -> TokenAmount {
        self.deposits.iter().map(|deposit| deposit.principal).sum()
    }

    pub(crate) fn get_liquid_balance(&self, terms: &Terms) -> (TokenAmount, usize) {
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

    pub(crate) fn should_close(&self) -> bool {
        self.deposits.is_empty() && self.cache.map_or(true, |cache| cache.interest == 0)
    }

    pub(crate) fn lock(&mut self) -> &mut Self {
        self.is_pending_withdraw = true;

        self
    }

    pub(crate) fn try_lock(&mut self) -> &mut Self {
        assert_not_locked(self);
        self.lock()
    }

    pub(crate) fn unlock(&mut self) -> &mut Self {
        self.is_pending_withdraw = false;

        self
    }

    pub(crate) fn claim(&mut self, remainder: u64, now: Timestamp) -> &mut Self {
        self.claim_remainder = remainder;
        self.cache = Some(JarCache {
            updated_at: now,
            interest: 0,
        });

        self
    }

    pub(crate) fn update_cache(&mut self, interest: TokenAmount, remainder: u64, now: Timestamp) {
        self.cache = Some(JarCache {
            updated_at: now,
            interest,
        });
        self.claim_remainder = remainder;
    }

    pub(crate) fn clean_up_deposits(&mut self, partition_index: usize) {
        if partition_index == self.deposits.len() {
            self.deposits.clear();
        } else {
            self.deposits.drain(..partition_index);
        }
    }

    pub(crate) fn apply(&mut self, companion: &JarCompanion) -> &mut Self {
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
}
