use std::collections::HashMap;

use near_sdk::{
    json_types::{U128, U64},
    near, Timestamp,
};

use crate::{ProductId, Timezone};

pub type JarId = u32;

pub type JarIdView = String;

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct JarView {
    pub id: JarIdView,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
}

#[derive(Debug, Clone, PartialEq)]
#[near(serializers=[json])]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<ProductId, U128>,
    pub total: U128,
}

impl Default for AggregatedTokenAmountView {
    fn default() -> Self {
        Self {
            detailed: HashMap::default(),
            total: U128(0),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
#[near(serializers=[json])]
pub struct AggregatedInterestView {
    pub amount: AggregatedTokenAmountView,
    pub timestamp: Timestamp,
}

/// The `JarTicket` struct represents a request to create a deposit jar for a corresponding product.
///
/// The data from this `JarTicket` is later combined with additional data, including the contract
/// account address, the recipient's account ID, the desired amount of tokens to deposit,
/// and the ID of the last jar created for the recipient. The concatenation of this data
/// forms a message that is then hashed using the SHA-256 algorithm. This resulting hash is used
/// to verify the authenticity of the data against an Ed25519 signature provided in the `ft_transfer_call` data.
#[derive(Clone, Debug)]
#[near(serializers=[json])]
pub struct DepositTicket {
    /// The unique identifier of the product for which the jar is intended to be created.
    /// This `product_id` links the request to the specific terms and conditions of the product that will govern the behavior of the jar.
    pub product_id: String,

    /// Specifies the expiration date of the ticket. The expiration date is measured in milliseconds
    /// since the Unix epoch. This property ensures that the request to create a jar is valid only
    /// until the specified timestamp. After this timestamp, the ticket becomes
    /// invalid and should not be accepted.
    pub valid_until: U64,

    /// An optional user timezone. Required for creating step jars.
    pub timezone: Option<Timezone>,
}
