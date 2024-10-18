use std::ops::{Deref, DerefMut};

use near_sdk::{
    borsh::{
        io::{Error, ErrorKind::InvalidData, Read},
        BorshDeserialize, BorshSerialize,
    },
    serde::{Deserialize, Serialize},
};

use crate::jar::model::{v1::JarV1, JarLastVersion};

pub type Jar = JarVersioned;

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, PartialEq)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
#[borsh(crate = "near_sdk::borsh")]
pub enum JarVersioned {
    V1(JarV1),
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to latest version
impl BorshDeserialize for JarVersioned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tag: u8 = BorshDeserialize::deserialize_reader(reader)?;

        let result = match tag {
            0 => JarVersioned::V1(BorshDeserialize::deserialize_reader(reader)?),
            // Add new versions here:
            _ => return Err(Error::new(InvalidData, format!("Unexpected variant tag: {tag:?}"))),
        };

        Ok(result)
    }
}

impl Deref for JarVersioned {
    type Target = JarLastVersion;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1(jar) => jar,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(jar) => jar, <- Add new version here
        }
    }
}

impl DerefMut for JarVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V1(jar) => jar,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(jar) => jar, <- Add new version here
        }
    }
}

impl From<JarV1> for JarVersioned {
    fn from(value: JarV1) -> Self {
        Self::V1(value)
    }
}
