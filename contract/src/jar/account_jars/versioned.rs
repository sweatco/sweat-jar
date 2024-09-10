use std::{
    io::{Error, ErrorKind::InvalidData, Read},
    ops::{Deref, DerefMut},
};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    jar::{
        account_jars::{v1::AccountJarsV1, AccountJarsLastVersion},
        model::AccountJarsLegacy,
    },
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
    score::AccountScore,
};

pub type AccountJars = AccountJarsVersioned;

#[derive(BorshSerialize, Debug, PartialEq)]
#[borsh(crate = "near_sdk::borsh")]
pub enum AccountJarsVersioned {
    V1(AccountJarsV1),
}

impl AccountJarsVersioned {
    pub fn score(&self) -> Option<&AccountScore> {
        if self.has_score_jars() {
            Some(&self.score)
        } else {
            None
        }
    }

    pub fn score_mut(&mut self) -> Option<&mut AccountScore> {
        if self.has_score_jars() {
            Some(&mut self.score)
        } else {
            None
        }
    }

    pub fn has_score_jars(&self) -> bool {
        self.score.is_valid()
    }
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to latest version
impl BorshDeserialize for AccountJarsVersioned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tag: u8 = BorshDeserialize::deserialize_reader(reader)?;

        let result = match tag {
            0 => AccountJarsVersioned::V1(BorshDeserialize::deserialize_reader(reader)?),
            // Add new versions here:
            _ => return Err(Error::new(InvalidData, format!("Unexpected variant tag: {tag:?}"))),
        };

        Ok(result)
    }
}

impl Default for AccountJarsVersioned {
    fn default() -> Self {
        Self::V1(AccountJarsV1::default())
    }
}

impl Deref for AccountJarsVersioned {
    type Target = AccountJarsLastVersion;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1(jars) => jars,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(jar) => jar, <- Add new version here
        }
    }
}

impl DerefMut for AccountJarsVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V1(jars) => jars,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(jar) => jar, <- Add new version here
        }
    }
}

impl From<AccountJarsLegacy> for AccountJars {
    fn from(value: AccountJarsLegacy) -> Self {
        Self::V1(value.into())
    }
}

impl From<AccountJarsNonVersioned> for AccountJars {
    fn from(value: AccountJarsNonVersioned) -> Self {
        Self::V1(value.into())
    }
}
