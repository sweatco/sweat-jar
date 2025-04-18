use near_sdk::near;

use crate::{Timestamp, TokenAmount};

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

/// A cached value that stores calculated interest based on the current state of the jar.
/// This cache is updated whenever properties that impact interest calculation change,
/// allowing for efficient interest calculations between state changes.
#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: TokenAmount,
}
