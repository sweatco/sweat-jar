use std::{
    io::{Error, ErrorKind::InvalidData, Read},
    ops::{Deref, DerefMut},
};

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
    serde::{Deserialize, Serialize},
    AccountId,
};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::{common::Timestamp, jar::model::JarCache};

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub struct JarLegacy {
    pub id: JarId,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: Timestamp,
    pub principal: TokenAmount,
    pub cache: Option<JarCache>,
    pub claimed_balance: TokenAmount,
    pub is_pending_withdraw: bool,
    pub is_penalty_applied: bool,
}

impl From<JarLegacy> for JarVersionedLegacy {
    #[mutants::skip]
    fn from(value: JarLegacy) -> Self {
        JarLegacyV1 {
            id: value.id,
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: value.created_at,
            principal: value.principal,
            cache: value.cache,
            claimed_balance: value.claimed_balance,
            is_pending_withdraw: value.is_pending_withdraw,
            is_penalty_applied: value.is_penalty_applied,
            claim_remainder: 0,
        }
        .into()
    }
}

#[near]
#[derive(Default, Debug, Clone)]
pub struct AccountLegacyV1 {
    pub last_id: JarId,
    pub jars: Vec<JarLegacy>,
}

#[near]
#[derive(Default, Clone)]
pub struct AccountLegacyV2 {
    pub last_id: JarId,
    pub jars: Vec<JarVersionedLegacy>,
}

impl From<AccountLegacyV1> for AccountLegacyV2 {
    #[mutants::skip]
    fn from(value: AccountLegacyV1) -> Self {
        AccountLegacyV2 {
            last_id: value.last_id,
            jars: value.jars.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, PartialEq)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
#[borsh(crate = "near_sdk::borsh")]
pub enum JarVersionedLegacy {
    V1(JarLegacyV1),
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to latest version
impl BorshDeserialize for JarVersionedLegacy {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tag: u8 = BorshDeserialize::deserialize_reader(reader)?;

        let result = match tag {
            0 => JarVersionedLegacy::V1(BorshDeserialize::deserialize_reader(reader)?),
            // Add new versions here:
            _ => return Err(Error::new(InvalidData, format!("Unexpected variant tag: {tag:?}"))),
        };

        Ok(result)
    }
}

impl Deref for JarVersionedLegacy {
    type Target = JarLegacyV1;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1(jar) => jar,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(jar) => jar, <- Add new version here
        }
    }
}

impl DerefMut for JarVersionedLegacy {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V1(jar) => jar,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(jar) => jar, <- Add new version here
        }
    }
}

impl From<JarLegacyV1> for JarVersionedLegacy {
    fn from(value: JarLegacyV1) -> Self {
        Self::V1(value)
    }
}

/// The `Jar` struct represents a deposit jar within the smart contract.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarLegacyV1 {
    /// The unique identifier for the jar.
    pub id: JarId,

    /// The account ID of the owner of the jar.
    pub account_id: AccountId,

    /// The product ID that describes the terms of the deposit associated with the jar.
    pub product_id: ProductId,

    /// The timestamp of when the jar was created, measured in milliseconds since Unix epoch.
    pub created_at: Timestamp,

    /// The principal amount of the deposit stored in the jar.
    pub principal: TokenAmount,

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
