use std::ops::{Deref, Sub};

use near_sdk::{env::block_timestamp_ms, near, Timestamp};

use crate::MS_IN_DAY;

pub type Day = Local;

/// Timestamp in UTC timezone
#[repr(transparent)]
#[near(serializers=[json, borsh])]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct UTC(pub Timestamp);

impl Default for UTC {
    fn default() -> Self {
        block_timestamp_ms().into()
    }
}

impl Deref for UTC {
    type Target = Timestamp;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Timestamp> for UTC {
    fn from(value: Timestamp) -> Self {
        Self(value)
    }
}

/// Timestamp in Local user timezone
#[repr(transparent)]
#[near(serializers=[json, borsh])]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Local(pub Timestamp);

impl Deref for Local {
    type Target = Timestamp;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Timestamp> for Local {
    fn from(value: Timestamp) -> Self {
        Self(value)
    }
}

impl From<usize> for Local {
    fn from(value: usize) -> Self {
        Self(value.try_into().unwrap())
    }
}

impl Sub for Local {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        (self.0 - rhs.0).into()
    }
}

pub trait TimeHelper {
    fn day(&self) -> Day;
    fn time(&self) -> Local;
}

impl TimeHelper for Local {
    fn day(&self) -> Day {
        (self.0 / MS_IN_DAY).into()
    }

    fn time(&self) -> Local {
        (self.0 % MS_IN_DAY).into()
    }
}
