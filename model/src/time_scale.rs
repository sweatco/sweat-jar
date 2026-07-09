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
//! - **Production**: Standard time constants (`MS_IN_DAY`, `MS_IN_YEAR`); the scale is
//!   a compile-time constant `1.0`, no storage is involved.
//! - **Integration tests**: Scaled values based on the global time scale.
//!
//! The scale is persisted under its own raw storage key, outside the `Contract` struct's
//! Borsh layout (the same pattern `near_plugins`' `AccessControllable` uses for its
//! `__acl` storage), so the integration-test build shares the production struct layout.
//! Each wasm instance lazily reads the key on first access and caches it in a
//! thread-local, so repeated `ms_in_day()`/`ms_in_year()` calls within one transaction
//! cost at most one storage read.
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
#[cfg(feature = "integration-methods")]
use std::cell::Cell;

use crate::{MS_IN_DAY, MS_IN_YEAR};

/// Raw storage key for the persisted time scale, deliberately outside the
/// `STATE` struct's Borsh layout. Must not collide with near-sdk's `STATE`
/// key, `near_plugins`' `__acl` prefix, or the contract's collection prefixes.
#[cfg(feature = "integration-methods")]
const TIME_SCALE_STORAGE_KEY: &[u8] = b"__time_scale";

#[cfg(feature = "integration-methods")]
thread_local! {
    /// Per-wasm-instance cache of the persisted time scale. `None` means "not
    /// loaded from storage yet"; each contract call starts fresh at `None`.
    static TIME_SCALE: Cell<Option<f64>> = const { Cell::new(None) };
}

/// Persist the global time scale and update the current instance's cache
/// (integration tests only). Storage-backed so the value survives across
/// contract calls without occupying a field in the `Contract` struct.
#[cfg(feature = "integration-methods")]
pub fn set_global_time_scale(scale: f64) {
    near_sdk::env::storage_write(TIME_SCALE_STORAGE_KEY, &scale.to_le_bytes());
    TIME_SCALE.with(|ts| ts.set(Some(scale)));
}

/// Get the time scale multiplier (integration tests only). Lazily reads the
/// persisted value on first access in this wasm instance, defaulting to `1.0`
/// when it was never set; subsequent calls hit the thread-local cache.
#[cfg(feature = "integration-methods")]
pub fn get_time_scale() -> f64 {
    TIME_SCALE.with(|ts| {
        if let Some(cached) = ts.get() {
            return cached;
        }

        let scale = near_sdk::env::storage_read(TIME_SCALE_STORAGE_KEY).map_or(1.0, |bytes| {
            f64::from_le_bytes(bytes.try_into().expect("Persisted time scale must be 8 bytes"))
        });
        ts.set(Some(scale));

        scale
    })
}

/// Get the time scale multiplier. In production there is no scaling, so this
/// is a constant `1.0` — no storage read.
#[cfg(not(feature = "integration-methods"))]
#[inline]
pub fn get_time_scale() -> f64 {
    1.0
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
