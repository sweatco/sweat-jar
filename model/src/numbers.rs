use near_sdk::{
    near,
    serde::{self, Deserialize, Deserializer, Serialize, Serializer},
};

#[near]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// `U32` serializes as a decimal string, so its ABI schema is a string,
/// mirroring near-sdk's `json_types` integers.
#[cfg(not(target_arch = "wasm32"))]
impl schemars::JsonSchema for U32 {
    fn schema_name() -> String {
        "U32".to_string()
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(generator)
    }
}
