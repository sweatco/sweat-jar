pub(crate) mod tests;
pub(crate) mod u32;
pub(crate) mod udecimal;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

/// Amount of fungible tokens
pub type TokenAmount = u128;

pub(crate) const MS_IN_SECOND: u64 = 1000;
pub(crate) const MS_IN_MINUTE: u64 = MS_IN_SECOND * 60;
pub(crate) const MINUTES_IN_YEAR: u64 = 365 * 24 * 60;
pub(crate) const MS_IN_YEAR: Duration = MINUTES_IN_YEAR * MS_IN_MINUTE;
