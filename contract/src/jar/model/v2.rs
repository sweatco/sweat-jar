use near_sdk::{near, AccountId};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::jar::model::common::{Deposit, JarCache};

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
