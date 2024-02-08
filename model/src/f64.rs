use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::Serialize,
};
use serde::Deserialize;

#[derive(Default, Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct F64(f64);

impl From<f64> for F64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Deref for F64 {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for F64 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialEq for F64 {
    fn eq(&self, other: &Self) -> bool {
        self.to_bits().eq(&other.to_bits())
    }
}

impl Eq for F64 {}

impl Hash for F64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_bits().hash(state)
    }
}

impl PartialOrd for F64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.to_bits().partial_cmp(&other.to_bits())
    }
}

impl Ord for F64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_bits().cmp(&other.to_bits())
    }
}
