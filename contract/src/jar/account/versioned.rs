use std::io::{Error, ErrorKind::InvalidData, Read};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::jar::account::v1::AccountV1;

pub type Account = AccountVersioned;

#[derive(BorshSerialize, Debug, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub enum AccountVersioned {
    V1(AccountV1),
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
        Self::V1(AccountV1::default())
    }
}
