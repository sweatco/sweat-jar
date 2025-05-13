use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

#[cfg(not(feature = "integration-test"))]
use near_sdk::AccountId;
use near_sdk::{json_types::U64, near};
#[cfg(feature = "integration-test")]
use nitka::near_sdk::AccountId;

use crate::{data::product::ProductId, signer::sha256, Timestamp, Timezone, TokenAmount};

/// The `DepositTicket` struct represents a request to create a deposit jar for a corresponding product.
///
/// The data from this `DepositTicket` is later combined with additional data, including the contract
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

#[derive(Clone, Debug)]
pub enum Purpose {
    Deposit,
    Restake,
}

impl Display for Purpose {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Purpose::Deposit => write!(f, "deposit"),
            Purpose::Restake => write!(f, "restake"),
        }
    }
}

pub struct DepositMessage(String);

impl DepositMessage {
    pub fn new(
        purpose: Purpose,
        contract_account_id: &AccountId,
        receiver_account_id: &AccountId,
        product_id: &ProductId,
        amount: TokenAmount,
        valid_until: Timestamp,
        nonce: u32,
    ) -> Self {
        Self(format!(
            "{purpose},{contract_account_id},{receiver_account_id},{product_id},{amount},{nonce},{valid_until}"
        ))
    }

    pub fn material(&self) -> &str {
        &self.0
    }

    pub fn sha256(&self) -> Vec<u8> {
        sha256(self.0.as_bytes())
    }
}

impl Display for DepositMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.clone())
    }
}

impl Deref for DepositMessage {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
