use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

pub(crate) const MINUTES_IN_YEAR: Duration = 365 * 24 * 60;
pub(crate) const MS_IN_MINUTE: u64 = 1000 * 60;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;
