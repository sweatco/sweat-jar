use std::{
    io::{Error, ErrorKind::InvalidData, Read},
    ops::{Deref, DerefMut},
};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::data::account::v1::AccountV1;

#[derive(BorshSerialize, Debug, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub enum AccountVersioned {
    V1(AccountV1),
    V1Sorted(AccountV1),
    V1SortedAndMerged(AccountV1),
}

impl AccountVersioned {
    pub fn new(account: AccountV1) -> Self {
        AccountVersioned::V1SortedAndMerged(account)
    }
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to the latest version
impl BorshDeserialize for AccountVersioned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tag: u8 = BorshDeserialize::deserialize_reader(reader)?;

        let result: AccountV1 = match tag {
            0 | 1 => {
                let account: AccountV1 = BorshDeserialize::deserialize_reader(reader)?;
                account.with_merged_deposits()
            }
            2 => BorshDeserialize::deserialize_reader(reader)?,
            // Add new versions here:
            _ => return Err(Error::new(InvalidData, format!("Unexpected variant tag: {tag:?}"))),
        };

        Ok(AccountVersioned::V1SortedAndMerged(result))
    }
}

impl Default for AccountVersioned {
    fn default() -> Self {
        Self::V1SortedAndMerged(AccountV1::default())
    }
}

impl Deref for AccountVersioned {
    type Target = AccountV1;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1SortedAndMerged(account) => account,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(account) => account, <- Add a new version here
            _ => panic!("Cannot deref this variant directly. Use V1SortedAndMerged."),
        }
    }
}

impl DerefMut for AccountVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V1SortedAndMerged(account) => account,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(account) => account, <- Add a new version here
            _ => panic!("Cannot deref this variant directly. Use V1SortedAndMerged."),
        }
    }
}
