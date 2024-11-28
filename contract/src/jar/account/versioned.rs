use std::{
    io::{Error, ErrorKind::InvalidData, Read},
    ops::{Deref, DerefMut},
};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::jar::account::{v1::AccountV1, Account};

#[derive(BorshSerialize, Debug, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub enum AccountVersioned {
    V1(AccountV1),
}

impl AccountVersioned {
    pub(crate) fn new(account: Account) -> Self {
        AccountVersioned::V1(account)
    }
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to latest version
impl BorshDeserialize for AccountVersioned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tag: u8 = BorshDeserialize::deserialize_reader(reader)?;

        let result = match tag {
            0 => AccountVersioned::V1(BorshDeserialize::deserialize_reader(reader)?),
            // Add new versions here:
            _ => return Err(Error::new(InvalidData, format!("Unexpected variant tag: {tag:?}"))),
        };

        Ok(result)
    }
}

impl Default for AccountVersioned {
    fn default() -> Self {
        Self::V1(Account::default())
    }
}

impl Deref for AccountVersioned {
    type Target = Account;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1(account) => account,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(account) => account, <- Add new version here
        }
    }
}

impl DerefMut for AccountVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V1(account) => account,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(account) => account, <- Add new version here
        }
    }
}
