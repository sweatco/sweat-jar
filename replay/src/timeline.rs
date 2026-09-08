//! Per-user DB slice -> engine [`Timeline`] synthesis.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use sweat_jar::replay::engine::{Action, Event, Score, Timeline};

use crate::parse::yocto_str_to_u128;

/// `seq` base for `step_packages`-derived events (jar_events keep their stored seq).
const SCORE_SEQ_BASE: u64 = 1_000_000_000;
/// `seq` base for `subscriptions`-derived events.
const SUB_SEQ_BASE: u64 = 2_000_000_000;

/// One account's on-chain summary alongside its synthesized timeline.
pub struct UserSlice {
    pub account_id: i64,
    pub near_account_id: String,
    /// Sum of every `jar_events` claim row's `amount` for this account.
    pub onchain_claimed: u128,
}

/// Reads every row for `account_id` from `jar_events`/`step_packages`/`subscriptions`
/// and builds a sorted engine [`Timeline`]. `near_account_id` comes from `users`
/// (error if the account is absent there).
pub fn load_user(conn: &Connection, account_id: i64) -> Result<(UserSlice, Timeline)> {
    let near_account_id: String = conn
        .query_row(
            "SELECT near_account_id FROM users WHERE account_id = ?1",
            [account_id],
            |r| r.get(0),
        )
        .with_context(|| format!("account {account_id} not in users table"))?;

    let mut events: Vec<Event> = Vec::new();
    let mut onchain_claimed: u128 = 0;

    // jar_events: stored seq is the tie-break. Claim rows collapse to one event
    // per distinct ts_ms (min seq among them), but every row's amount is summed.
    let jar_rows: Vec<(u64, u64, String, String, String)> = conn
        .prepare(
            "SELECT ts_ms, seq, event_type, product_id, amount FROM jar_events \
             WHERE account_id = ?1 ORDER BY ts_ms, seq",
        )?
        .query_map([account_id], |r| {
            Ok((
                r.get::<_, i64>(0)? as u64,
                r.get::<_, i64>(1)? as u64,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut claim_seq: BTreeMap<u64, u64> = BTreeMap::new();
    for (ts_ms, seq, event_type, product_id, amount) in jar_rows {
        match event_type.as_str() {
            "deposit" => events.push(Event {
                ts_ms,
                seq,
                action: Action::Deposit {
                    product_id,
                    amount: yocto_str_to_u128(&amount)?,
                },
            }),
            "withdraw" => events.push(Event {
                ts_ms,
                seq,
                action: Action::Withdraw { product_id },
            }),
            "restake" => events.push(Event {
                ts_ms,
                seq,
                action: Action::Restake { product_id },
            }),
            "claim" => {
                onchain_claimed = onchain_claimed
                    .checked_add(yocto_str_to_u128(&amount)?)
                    .context("onchain_claimed overflow")?;
                claim_seq
                    .entry(ts_ms)
                    .and_modify(|s| *s = (*s).min(seq))
                    .or_insert(seq);
            }
            other => bail!("unknown jar_events event_type {other:?} for account {account_id}"),
        }
    }
    for (ts_ms, seq) in claim_seq {
        events.push(Event {
            ts_ms,
            seq,
            action: Action::Claim,
        });
    }

    // step_packages -> RecordScore, seq = SCORE_SEQ_BASE + row_index (ts order).
    let step_rows: Vec<(u64, i64)> = conn
        .prepare("SELECT ts_ms, steps FROM step_packages WHERE account_id = ?1 ORDER BY ts_ms")?
        .query_map([account_id], |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (i, (ts_ms, steps)) in step_rows.into_iter().enumerate() {
        let score: Score = u16::try_from(steps).unwrap_or(u16::MAX);
        events.push(Event {
            ts_ms,
            seq: SCORE_SEQ_BASE + i as u64,
            action: Action::RecordScore(score),
        });
    }

    // subscriptions -> SetIncreasedScoreCap, seq = SUB_SEQ_BASE + row_index.
    let sub_rows: Vec<(u64, i64)> = conn
        .prepare("SELECT ts_ms, active FROM subscriptions WHERE account_id = ?1 ORDER BY ts_ms")?
        .query_map([account_id], |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (i, (ts_ms, active)) in sub_rows.into_iter().enumerate() {
        events.push(Event {
            ts_ms,
            seq: SUB_SEQ_BASE + i as u64,
            action: Action::SetIncreasedScoreCap(active == 1),
        });
    }

    let slice = UserSlice {
        account_id,
        near_account_id,
        onchain_claimed,
    };
    Ok((slice, Timeline { events }.sorted()))
}
