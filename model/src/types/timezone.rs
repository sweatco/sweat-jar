use std::ops::{Deref, DerefMut};

use near_sdk::{
    env::{block_timestamp_ms, panic_str},
    near, Timestamp,
};

use crate::{Day, Local, TimeHelper, MS_IN_HOUR, UTC};

/// Timezone described as time shift from UTC in ms
#[repr(transparent)]
#[near(serializers=[json, borsh])]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Timezone(i64);

impl Timezone {
    pub const fn invalid() -> Self {
        Self(i64::MIN)
    }

    pub const fn is_valid(&self) -> bool {
        self.0 != i64::MIN
    }

    pub const fn hour_shift(hour: i64) -> Self {
        // MS_IN_HOUR won't wrap
        #[allow(clippy::cast_possible_wrap)]
        Self(MS_IN_HOUR as i64 * hour)
    }

    pub fn adjust(&self, timestamp: UTC) -> Local {
        let timestamp: Timestamp = (i128::from(self.0) + i128::from(timestamp.0))
            .try_into()
            .unwrap_or_else(|_| {
                panic_str(&format!(
                    "Failed to adjust timestamp: {timestamp:?} for timezone: {}",
                    self.0
                ))
            });
        timestamp.into()
    }

    pub fn now(&self) -> Local {
        self.adjust(UTC(block_timestamp_ms()))
    }

    /// Return current day index
    pub fn today(&self) -> Day {
        self.now().day()
    }

    pub fn time(&self) -> Local {
        self.now().time()
    }
}

impl Deref for Timezone {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Timezone {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
