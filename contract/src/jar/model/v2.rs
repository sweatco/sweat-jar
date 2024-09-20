use near_sdk::{near, AccountId};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::{common::Timestamp, jar::model::JarCache};

/// The `Jar` struct represents a deposit jar within the smart contract.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarV2 {
    /// The unique identifier for the jar.
    pub id: JarId,

    /// The product ID that describes the terms of the deposit associated with the jar.
    pub product_id: ProductId,

    /// Deposits stored in the jar.
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

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Deposit {
    /// The timestamp of when the top-up was added, measured in milliseconds since Unix epoch.
    pub created_at: Timestamp,

    /// The amount of the top-up.
    pub principal: TokenAmount,
}

impl Deposit {
    pub fn new(created_at: Timestamp, principal: TokenAmount) -> Self {
        Deposit { created_at, principal }
    }
}
