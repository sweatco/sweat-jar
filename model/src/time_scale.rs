//! Time scaling for integration tests
//!
//! This module provides time scaling functionality that allows integration tests to run faster
//! by compressing time. For example, with a scale of 1/24, one day passes in one hour.
//!
//! # Architecture
//!
//! ## Global Accessor Functions (Recommended)
//!
//! Use `ms_in_day()` and `ms_in_year()` functions anywhere in your code, similar to `env::block_timestamp_ms()`.
//! These functions automatically return:
//! - **Production**: Standard time constants (`MS_IN_DAY`, `MS_IN_YEAR`)
//! - **Integration tests**: Scaled values based on the global time scale
//!
//! The time scale is stored in thread-local storage and automatically initialized when the contract
//! is deserialized or when `set_time_scale()` is called.
//!
//! ## Example
//!
//! ```ignore
//! use sweat_jar_model::ms_in_day;
//!
//! // In production: returns 86_400_000 ms
//! // In tests with scale=1/24: returns 3_600_000 ms (1 hour)
//! let one_day = ms_in_day();
//! ```
//!
use crate::{MS_IN_DAY, MS_IN_YEAR};

#[cfg(feature = "integration-methods")]
use std::cell::Cell;

#[cfg(feature = "integration-methods")]
thread_local! {
    /// Thread-local storage for time scale in integration tests.
    /// This allows us to have a global accessor similar to env::block_timestamp_ms()
    static TIME_SCALE: Cell<f64> = Cell::new(1.0);
}

/// Set the global time scale for the current thread (integration tests only).
/// This should be called when the contract is deserialized or when set_time_scale is called.
#[cfg(feature = "integration-methods")]
pub fn set_global_time_scale(scale: f64) {
    TIME_SCALE.with(|ts| ts.set(scale));
}

/// Get the time scale multiplier (integration tests only).
#[cfg(feature = "integration-methods")]
pub fn get_time_scale() -> f64 {
    TIME_SCALE.with(|ts| ts.get())
}

/// Returns the number of milliseconds in a day, scaled for integration tests.
/// In production: returns standard `MS_IN_DAY` constant.
/// In integration tests: returns scaled value based on global time scale.
#[cfg(not(feature = "integration-methods"))]
#[inline]
pub fn ms_in_day() -> u64 {
    MS_IN_DAY
}

#[cfg(feature = "integration-methods")]
#[inline]
pub fn ms_in_day() -> u64 {
    ((MS_IN_DAY as f64) * get_time_scale()) as u64
}

/// Returns the number of milliseconds in a year, scaled for integration tests.
/// In production: returns standard `MS_IN_YEAR` constant.
/// In integration tests: returns scaled value based on global time scale.
#[cfg(not(feature = "integration-methods"))]
#[inline]
pub fn ms_in_year() -> u64 {
    MS_IN_YEAR
}

#[cfg(feature = "integration-methods")]
#[inline]
pub fn ms_in_year() -> u64 {
    ((MS_IN_YEAR as f64) * get_time_scale()) as u64
}
