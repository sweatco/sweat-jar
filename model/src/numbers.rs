use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{self, Deserialize, Deserializer, Serialize, Serializer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize, Hash)]
pub struct U32(pub u32);

impl From<u32> for U32 {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Serialize for U32 {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for U32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(Self(
            str::parse::<u32>(&s).map_err(|err| serde::de::Error::custom(err.to_string()))?,
        ))
    }
}
