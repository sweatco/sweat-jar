use std::collections::HashMap;

use near_sdk::near;

use crate::{Timestamp, TokenAmount};

/// The `Jar` struct represents a deposit jar within the smart contract.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct Jar {
    pub deposits: Vec<Deposit>,
    pub cache: Option<JarCache>,
    pub is_locked: bool,
    pub claim_remainder: u64,
}

#[allow(clippy::option_option)]
#[near(serializers=[json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct JarCompanion {
    pub deposits: Option<Vec<Deposit>>,
    pub cache: Option<Option<JarCache>>,
    pub is_locked: Option<bool>,
    pub claim_remainder: Option<u64>,
}

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Deposit {
    pub created_at: Timestamp,
    pub principal: TokenAmount,
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

impl Jar {
    pub fn sort_deposits(&mut self) {
        self.deposits.sort_by_key(|deposit| deposit.created_at);
    }

    pub fn merge_deposits(&mut self) {
        let mut merged_deposits: HashMap<Timestamp, TokenAmount> = HashMap::new();
        for deposit in &self.deposits {
            *merged_deposits.entry(deposit.created_at).or_insert(0) += deposit.principal;
        }

        self.deposits = merged_deposits
            .into_iter()
            .map(|(created_at, principal)| Deposit::new(created_at, principal))
            .collect();
    }
}
