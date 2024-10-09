use near_sdk::near;
use sweat_jar_model::{TokenAmount, UDecimal};

use crate::{
    common::{Duration, Timestamp},
    jar::model::JarCache,
};

/// The `Jar` struct represents a deposit jar within the smart contract.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct JarV2 {
    pub deposits: Vec<Deposit>,
    pub cache: Option<JarCache>,
    pub claimed_balance: TokenAmount,
    pub is_pending_withdraw: bool,
    pub claim_remainder: u64,
}

#[near(serializers=[json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct JarV2Companion {
    pub deposits: Option<Vec<Deposit>>,
    pub cache: Option<Option<JarCache>>,
    pub claimed_balance: Option<TokenAmount>,
    pub is_pending_withdraw: Option<bool>,
    pub claim_remainder: Option<u64>,
}

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Deposit {
    pub created_at: Timestamp,
    pub principal: TokenAmount,
}

impl JarV2 {
    pub(crate) fn lock(&mut self) -> &mut Self {
        self.is_pending_withdraw = true;

        self
    }

    pub(crate) fn unlock(&mut self) -> &mut Self {
        self.is_pending_withdraw = false;

        self
    }

    pub(crate) fn claim(&mut self, claimed_amount: TokenAmount, remainder: u64, now: Timestamp) -> &mut Self {
        self.claimed_balance += claimed_amount;
        self.claim_remainder = remainder;
        self.cache = Some(JarCache {
            updated_at: now,
            interest: 0,
        });

        self
    }

    pub(crate) fn apply(&mut self, companion: JarV2Companion) -> &mut Self {
        if let Some(claim_remainder) = companion.claim_remainder {
            self.claim_remainder = claim_remainder;
        }

        if let Some(claimed_balance) = companion.claimed_balance {
            self.claimed_balance = claimed_balance;
        }

        if let Some(cache) = companion.cache {
            self.cache = cache;
        }

        if let Some(deposits) = companion.deposits {
            self.deposits = deposits;
        }

        if let Some(is_pending_withdraw) = companion.is_pending_withdraw {
            self.is_pending_withdraw = is_pending_withdraw;
        }

        self
    }
}

impl Deposit {
    pub(crate) fn new(created_at: Timestamp, principal: TokenAmount) -> Self {
        Self { created_at, principal }
    }

    pub(crate) fn is_liquidable(&self, now: Timestamp, term: Duration) -> bool {
        now - self.created_at > term
    }
}
