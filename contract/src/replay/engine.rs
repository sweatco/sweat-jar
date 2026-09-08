//! Reusable timeline-execution engine for account replays.
//!
//! [`run_timeline`] runs an ordered list of [`Event`]s against a fresh
//! in-process contract on the current thread, returning a [`ReplayOutcome`].
//! Contract panics are caught and surfaced as [`ReplayStatus::Error`] — the
//! function never panics.

use std::panic::{catch_unwind, AssertUnwindSafe};

use near_sdk::{json_types::Base64VecU8, AccountId, PromiseOrValue};
pub use sweat_jar_model::data::product::Product;
use sweat_jar_model::{
    api::{AccountApi, ClaimApi, RestakeApi, WithdrawApi},
    data::{account::features::Feature, deposit::DepositTicket},
};
pub use sweat_jar_model::{Score, UTC};

use crate::{
    common::{env::test_env_ext, testing::Context},
    migration::api::store_account_raw,
};

/// A single account interaction to replay.
#[derive(Clone, Debug)]
pub enum Action {
    RecordScore(Score),
    Deposit { product_id: String, amount: u128 },
    Withdraw { product_id: String },
    Restake { product_id: String },
    SetIncreasedScoreCap(bool),
    Claim,
}

impl Action {
    /// Tie-break rank for events sharing a millisecond: scores land first, then
    /// state-changing calls, then claims (so a claim sees up-to-date state).
    pub fn rank(&self) -> u8 {
        match self {
            Action::RecordScore(_) => 0,
            Action::Deposit { .. }
            | Action::Withdraw { .. }
            | Action::Restake { .. }
            | Action::SetIncreasedScoreCap(_) => 1,
            Action::Claim => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub ts_ms: u64,
    pub seq: u64,
    pub action: Action,
}

#[derive(Default)]
pub struct Timeline {
    pub events: Vec<Event>,
}

impl Timeline {
    /// Sorts events by `(ts_ms, rank, seq)` in place.
    pub fn sorted(mut self) -> Self {
        self.events.sort_by_key(|e| (e.ts_ms, e.action.rank(), e.seq));
        self
    }
}

/// Opaque baseline: the raw borsh bytes of an `AccountVersioned`, or `None` for
/// a fresh account.
pub struct Baseline {
    pub account_id: AccountId,
    pub raw_account: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum ReplayStatus {
    Ok,
    Error(String),
}

#[derive(Debug)]
pub struct ReplayOutcome {
    pub total_claimed: u128,
    pub per_claim: Vec<(u64, u128)>,
    pub status: ReplayStatus,
}

/// Runs the timeline against a fresh in-process contract on the current thread.
///
/// Resets thread-local mock storage before starting (a fresh [`Context`] takes
/// the mock storage). Never panics: contract panics are caught and surfaced as
/// [`ReplayStatus::Error`].
///
/// Note: `Action::Withdraw` withdraws the entire liquid principal of the
/// product's jar — the contract has no partial-amount withdraw — so historical
/// partial withdrawals are a known divergence.
pub fn run_timeline(baseline: Baseline, products: &[Product], window_start_ms: u64, timeline: Timeline) -> ReplayOutcome {
    test_env_ext::set_test_log_events(false);

    let account_id = baseline.account_id.clone();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut context = Context::new(admin()).with_products(products);

        if let Some(raw) = &baseline.raw_account {
            store_account_raw(account_id.clone(), Base64VecU8(raw.clone()));
        }
        context.set_block_timestamp_in_ms(window_start_ms);

        let mut total_claimed = 0u128;
        let mut per_claim: Vec<(u64, u128)> = Vec::new();

        for event in timeline.events {
            context.set_block_timestamp_in_ms(event.ts_ms);
            match event.action {
                Action::RecordScore(score) => {
                    context.switch_account_to_operator();
                    context
                        .contract()
                        .record_score(vec![(account_id.clone(), vec![(score, UTC(event.ts_ms))])]);
                }
                Action::Deposit { product_id, amount } => {
                    let ticket = DepositTicket {
                        product_id,
                        valid_until: 0.into(),
                        timezone: None,
                    };
                    context.switch_account_to_ft_contract_account();
                    context.contract().deposit(account_id.clone(), ticket, amount, None);
                }
                Action::Withdraw { product_id } => {
                    context.switch_account(&account_id);
                    let _ = context.contract().withdraw(product_id);
                }
                Action::Restake { product_id } => {
                    context.switch_account(&account_id);
                    let ticket = DepositTicket {
                        product_id: product_id.clone(),
                        valid_until: 0.into(),
                        timezone: None,
                    };
                    let _ = context.contract().restake(product_id, ticket, None, None);
                }
                Action::SetIncreasedScoreCap(enabled) => {
                    context.switch_account_to_operator();
                    context
                        .contract()
                        .set_feature_enabled(account_id.clone(), Feature::IncreasedScoreCap, enabled);
                }
                Action::Claim => {
                    context.switch_account(&account_id);
                    if let PromiseOrValue::Value(claimed) = context.contract().claim_total(None) {
                        let amount = claimed.get_total().0;
                        total_claimed += amount;
                        per_claim.push((event.ts_ms, amount));
                    }
                }
            }
        }
        (total_claimed, per_claim)
    }));

    match result {
        Ok((total_claimed, per_claim)) => ReplayOutcome {
            total_claimed,
            per_claim,
            status: ReplayStatus::Ok,
        },
        Err(e) => {
            let msg = e
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            ReplayOutcome {
                total_claimed: 0,
                per_claim: Vec::new(),
                status: ReplayStatus::Error(msg),
            }
        }
    }
}

fn admin() -> AccountId {
    "admin.near".parse().unwrap()
}
