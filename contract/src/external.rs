use near_sdk::{ext_contract, is_promise_success};

use crate::*;

// TODO: maybe calculate in runtime
pub(crate) const GAS_FOR_AFTER_TRANSFER: u64 = 20_000_000_000_000;
