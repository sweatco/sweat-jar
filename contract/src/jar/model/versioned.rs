use std::ops::{Deref, DerefMut};

use near_sdk::{
    borsh::{
        io::{Error, ErrorKind::InvalidData, Read},
        BorshDeserialize, BorshSerialize,
    },
    env::panic_str,
    serde::{Deserialize, Serialize},
    AccountId,
};
use sweat_jar_model::{jar::JarId, ProductId, Score, TokenAmount};

use crate::{
    common::Timestamp,
    jar::model::{
        v1::JarV1,
        v2::{Deposit, JarV2},
        JarCache, JarLastVersion,
    },
    product::model::Product,
};

pub type Jar = JarVersioned;

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, PartialEq)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
#[borsh(crate = "near_sdk::borsh")]
pub enum JarVersioned {
    V1(JarV1),
    V2(JarV2),
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to latest version
impl BorshDeserialize for JarVersioned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tag: u8 = BorshDeserialize::deserialize_reader(reader)?;

        let result = match tag {
            0 => JarVersioned::V1(BorshDeserialize::deserialize_reader(reader)?),
            1 => JarVersioned::V2(BorshDeserialize::deserialize_reader(reader)?),
            // Add new versions here:
            _ => return Err(Error::new(InvalidData, format!("Unexpected variant tag: {tag:?}"))),
        };

        Ok(result)
    }
}

impl JarVersioned {
    pub fn create(id: JarId, product_id: ProductId, principal: TokenAmount, created_at: Timestamp) -> Self {
        JarLastVersion {
            id,
            product_id,
            deposits: vec![Deposit::new(created_at, principal)],
            cache: None,
            claimed_balance: 0,
            is_pending_withdraw: false,
            is_penalty_applied: false,
            claim_remainder: 0,
        }
        .into()
    }

    pub fn locked(&self) -> Self {
        JarLastVersion {
            is_pending_withdraw: true,
            ..self.deref().clone()
        }
        .into()
    }

    pub fn unlocked(&self) -> Self {
        JarLastVersion {
            is_pending_withdraw: false,
            ..self.deref().clone()
        }
        .into()
    }

    pub fn with_id(mut self, id: JarId) -> Self {
        self.id = id;
        self
    }

    pub fn withdrawn(&self, score: &[Score], product: &Product, now: Timestamp) -> Self {
        JarV2 {
            principal: self.principal - withdrawn_amount,
            cache: Some(JarCache {
                updated_at: now,
                interest: self.get_interest(score, product, now).0,
            }),
            ..self.deref().clone()
        }
        .into()
    }
}

impl Deref for JarVersioned {
    type Target = JarLastVersion;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V2(jar) => jar,
            _ => panic_str("Cannot deref outdated version"),
        }
    }
}

impl DerefMut for JarVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V2(jar) => jar,
            _ => panic_str("Cannot deref outdated version"),
        }
    }
}

impl From<JarV1> for JarVersioned {
    fn from(value: JarV1) -> Self {
        Self::V1(value)
    }
}

impl From<JarV2> for JarVersioned {
    fn from(value: JarV2) -> Self {
        Self::V2(value)
    }
}
