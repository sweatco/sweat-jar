use near_sdk::{
    json_types::U64,
    near,
    serde::{Deserialize, Serialize},
    AccountId,
};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::common::Timestamp;

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
