# `replay` — multi-user reconciliation

## What it does

For every Sweat user, `replay` re-simulates their jar activity over a fixed
window against the local contract and compares the **replayed total claimed**
against the **on-chain total claimed** recorded by the indexer. It emits one CSV
row per account with the signed delta so accounts whose interest math diverges
can be found. The full event history is staged in a local SQLite database, built
once and read concurrently by the reconciliation run.

This is the population-scale counterpart of the single-account
`replay_account_history` golden test in `contract/src/replay.rs`.

## Window

| bound | value | meaning |
|-------|-------|---------|
| `H` (start) | `1774017710156` ms | block `190375496`, `2026-03-20T14:41:50.156Z` — baseline account state is "as of `H`" |
| `T_end` (end) | `1788174657961` ms | `2026-08-31T11:10:57.961Z` |

An event row is kept **iff `H < ts <= T_end`** (strictly after `H`, up to and
including `T_end`). The bounds live in `replay/src/parse.rs` (`H_MS`,
`T_END_MS`) and are written into the `meta` table at build time.

## Workflow

Build once with the **`release-replay`** profile, then run the three subcommands
in order. `release-replay` is an optimized build with `panic = "unwind"` — the
plain `release` profile sets `panic = "abort"` (for the contract wasm), which
disables the per-account panic recovery in `run` and makes the first contract
panic kill the whole run.

```sh
cargo build -p replay --profile release-replay
BIN=./target/release-replay/replay

# 1. Refresh the on-chain product catalogue (committed to the repo).
#    Calls get_products() on v2.jars.sweat via mainnet JSON RPC.
$BIN fetch-products --out test_data/products.json

# 2. Ingest the source CSVs (+ optional snapshots.ndjson) into SQLite.
#    Re-runnable: each table is cleared and re-populated. (DELETE doesn't
#    reclaim pages, so a re-run keeps replay.db at its high-water mark;
#    `rm replay.db` first if you want a smaller file.)
$BIN build-db --db replay.db --test-data-dir test_data

# 3. Reconcile and write the report. Set --threads to your core count.
$BIN run --db replay.db --out reconciliation.csv --products test_data/products.json --threads "$(nproc 2>/dev/null || sysctl -n hw.ncpu)"
```

`build-db` streams the full `step_packages.csv` (~285M rows / ~10 GB) even with
`--sample`, so it takes tens of minutes; `replay.db` lands around 15–25 GB. For
dev iteration, skip that file: `build-db --sample 50 --only users,jar_events,subscriptions`
then `run --sample 50` (no score events, so score-based totals are incomplete —
fine for a plumbing check).

## `build-db` flags

| flag | default | meaning |
|------|---------|---------|
| `--db <path>` | — | SQLite file to create / append to (required) |
| `--test-data-dir <dir>` | `test_data` | directory holding the source CSVs |
| `--only <t1,t2,...>` | all | ingest only these tables: `users`, `jar_events`, `step_packages`, `boosted_step_packages`, `subscriptions`, `snapshots` (an unknown name is rejected) |
| `--accounts <file>` | — | keep only these `account_id`s; one integer per line, blank lines and `#` comments ignored |
| `--sample <N>` | — | keep only the first `N` accounts from `users.csv` (file order) and their event rows |

**Keep-set precedence:** `--accounts` beats `--sample` beats "all". If
`--accounts` is given, `--sample` is ignored.

`subscriptions` is ingested from `max_subscriptions.csv`. `snapshots` resolves
each line's `near_account_id` against the `users` table, so ingest `users`
(re)first or in the same run; `snapshots.ndjson` absent is a no-op.

## `run` flags

| flag | default | meaning |
|------|---------|---------|
| `--db <path>` | — | replay database (opened read-only, required) |
| `--out <path>` | `reconciliation.csv` | report CSV |
| `--products <path>` | `test_data/products.json` | product catalogue from `fetch-products` |
| `--threads <N>` | CPU count | worker threads; workers are named `replay-worker-*`, the CSV writer `replay-writer` |
| `--shard <i/n>` | — | process only accounts where `account_id.rem_euclid(n) == i` — for multi-process / multi-machine fan-out over one read-only DB |
| `--accounts <file>` | — | same format as `build-db --accounts`; restrict the worklist |
| `--sample <N>` | — | process only the first `N` accounts of the (already filtered) worklist |
| `--tolerance <f>` | `1e-6` | an `ok` row with `|rel_delta| > tolerance` is counted in `over_tolerance` |
| `--archival` | off | fetch each account's block-`H` state live from a NEAR archival node (`get_account` view at block 190375496) instead of the local `snapshots` table |
| `--archival-rpc-url <url>` | `https://archival-rpc.mainnet.fastnear.com` | archival endpoint used with `--archival` |

**`--archival`** removes the need for a pre-populated `snapshots` table — every
account gets a real baseline. Cost: one RPC round-trip per account (3 retries on
transient failure), so ~0.2 s/account with `--threads 12`; a full 1.2M-account
run is many hours. Use `--shard i/n` to spread it across machines, or `--sample`
/ `--accounts` for spot checks. `view_state` over the whole contract is refused
by public archival nodes ("state too large"), so the per-account view call is
the only route.

## Input CSVs (`test_data/`)

| file | on-chain / notes | key column |
|------|------------------|-----------|
| `users.csv` | `near_account_id` is the on-chain `AccountId`; `sweatcoin_user_id` is not ingested | `account_id` |
| `jar_events.csv` | `event_type` ∈ `deposit`/`claim`/`withdraw`/`restake`/`merge`; `near_block_timestamp` is ISO-8601 ms `Z`; `amount` is the token base unit as a decimal string | `account_id` |
| `step_packages.csv` | `account_id,created_at,steps,yesterday_steps`; `created_at` is `YYYY-MM-DD HH:MM:SS UTC`; each package replays as one `record_score` with `(steps, created_at)` **and** `(yesterday_steps, created_at−24h)` (each if `>0`), matching the oracle's `to_args`; `steps` clamped to `65535` | `account_id` |
| `boosted_step_packages.csv` | `account_id,created_at,steps,status,processing_type`; only `status = executed` rows are ingested; each replays as one `record_score` with a single `(steps, created_at)` increment | `account_id` |
| `max_subscriptions.csv` | `action_type` ∈ `subscribed`/`expired` → `active` `1`/`0` | `user_id` (== `account_id`) |

`account_id` is the same integer across all files. Optional
**`snapshots.ndjson`** — one JSON object per line with `near_account_id`,
`account_state`, and `products_referenced` (mirrors
`test_data/account_full_state_190375496.json`). **Not produced yet** — see
Limitations.

## SQLite schema

Tables (verbatim from `replay/src/db/schema.rs`):

```
users                 (account_id PK, near_account_id)
jar_events            (account_id, ts_ms, seq, event_type, product_id, amount)
step_packages         (account_id, ts_ms, steps, yesterday_steps)
boosted_step_packages (account_id, ts_ms, steps)   -- executed rows only
subscriptions         (account_id, ts_ms, active)  -- subscribed=1, expired=0
snapshots             (account_id PK, state_json)  -- verbatim ndjson line
meta                  (key PK, value)
```

`meta` rows: `window_h_ms`, `window_t_end_ms`, `built_at` (build time, epoch ms).

Notes:
- Indexes (`ix_jar_events_acct`, `ix_step_packages_acct`,
  `ix_boosted_step_packages_acct`, `ix_subscriptions_acct`) are created
  **after** the bulk load.
- Build connection runs `PRAGMA synchronous = OFF; PRAGMA journal_mode =
  MEMORY;`; one transaction per table (the `step_packages` load commits and
  reopens every 1M rows).
- `jar_events.amount` is **TEXT** — a u128 decimal string, which overflows a
  SQLite INTEGER.
- Ingest drops `merge` rows and the `deposit_ids` / `fee_amount` /
  `product_name` columns; every event row outside `(H, T_end]` is dropped.
- `run` opens each worker connection `SQLITE_OPEN_READ_ONLY`.

## `reconciliation.csv`

```
account_id,near_account_id,calculated_total_claim,actual_total_claim,delta,rel_delta,n_claims,status
```

| column | meaning |
|--------|---------|
| `calculated_total_claim` | Σ of the amounts returned by each replayed `claim_total(None)` |
| `actual_total_claim` | Σ `jar_events.amount` where `event_type = 'claim'` in the window |
| `delta` | `calculated_total_claim − actual_total_claim` (signed, i128) |
| `rel_delta` | `delta / actual_total_claim` (`0.0` when actual is `0`) |
| `n_claims` | number of distinct claim timestamps replayed |
| `status` | `ok` \| `error:<msg>` \| `no_baseline` |

`status` values: `ok` — replay succeeded and a baseline snapshot was present;
`no_baseline` — replay succeeded but the account had no snapshot (started from an
empty account; informational); `error:<msg>` — the engine or snapshot parse
panicked / failed for this account (`calculated` is `0`, `delta` is
`-actual`).

`run` prints a summary to stdout:

```
processed <n> | ok <n> | error <n> | no_baseline <n> | over_tolerance <n>
sum_calculated <n> | sum_actual <n>
```

## How sharding + threads compose

`--threads` parallelises **within one process**: a shared work queue of
`account_id`s fed to N named workers, each replaying users independently
(near-sdk's mock env is thread-local, so a worker resets it per job).

`--shard i/n` splits the **user set across N processes / machines**, each
processing a disjoint slice (`account_id % n == i`) and writing its own CSV.
Concatenate the shard outputs afterwards and de-duplicate the header row. Both
compose: run `--shard i/n --threads T` on each machine.

## Known limitations

- **Baseline snapshots.** Two ways to give an account its block-`H` state:
  `run --archival` (fetches it live from an archival node, no setup) or a
  pre-populated `snapshots` table / `snapshots.ndjson` (fast, offline, but the
  bulk extractor that writes it is not built yet). Without either, an account
  that held jars before `H` replays without that prior state — see below.
- **Without a baseline (`run` with no `--archival` and no `snapshots` rows):**
  a claim/withdraw against a jar we don't have is caught and the row is marked
  `status = no_baseline` (`calculated = 0`, large negative `delta`). A
  no-baseline account whose replay panics for an unrelated reason (e.g. a
  score-based deposit with no timezone) shows `error:` instead. Such rows are
  informational only — use `--archival` (or populate `snapshots`) to reconcile
  pre-`H` holders.
- **No bulk snapshot extractor.** `run --archival` fetches baselines one account
  at a time. There is no tool yet to bulk-populate the `snapshots` table /
  `snapshots.ndjson` for fast repeated offline runs — a `fetch-snapshots`
  subcommand wrapping the same archival call is the obvious next step.
- **Product config is current-state only.** `fetch-products` captures
  `get_products()` as of now, not as it stood during the window. APY, cap, and
  enable/disable changes inside `(H, T_end]` are not modeled.
- **`withdraw` replays as a full-liquid-principal withdrawal.** The current
  contract `withdraw(product_id)` has no partial-amount form, so
  `jar_events.amount` for a withdraw row is informational (the engine does not
  read it). Historical partial withdrawals will diverge.
- **`merge` events are ignored** — a no-op in the merged-jars contract model;
  dropped at ingest.
- **Signatures are not verified.** Deposits replay unsigned; `load_products`
  strips `public_key` from every product.
- **`increased_apy` is taken from the baseline snapshot and never toggled.**
  Only `increased_score_cap` is driven by subscription events
  (`SetIncreasedScoreCap`).
- **Subscription events outside `(H, T_end]` are dropped at ingest** like every
  other event row. Pre-`H` subscription state must come from the baseline
  snapshot.
- **Score interest depends on matching the oracle's `record_score` cadence.**
  Each `step_packages` row replays as one `record_score` with `(steps, created_at)`
  + `(yesterday_steps, created_at−24h)`, and each executed `boosted_step_packages`
  row adds another. Days with no package earn nothing (the oracle skips them
  too). What is *not* modeled: on-chain the oracle transaction executes some
  time after `created_at`, so an increment that arrives ≥2 days late is
  discarded by the contract (`ScoreIncrementProcessor`); the replay applies
  every increment at `created_at`, so accounts whose oracle was delayed
  reconcile slightly high. The `BoosterApplication` / `apply_booster` path (the
  `score.booster` field) is not modeled at all.

## Build requirements

- `rusqlite` uses the `bundled` feature (compiles SQLite from C source), so a C
  compiler must be available wherever `replay` is built.
- `replay` depends on `sweat_jar` with the `replay-engine` feature, which pulls
  `near-sdk/unit-testing` — a **host-only** build.
- `replay` is **not** in the workspace `default-members`, so a plain
  `cargo build` / `cargo test` and CI are unaffected. Build it explicitly with
  `-p replay`.

## Testing

```sh
cargo test -p replay
```

Covers CSV/timestamp/amount parsers, ingest rules (window filter, `merge` drop,
`steps` clamp, keep-set precedence, `meta` rows), timeline synthesis
(`rank`/`seq` ordering, claim collapse), single-user reconcile, the threaded
`run`, and thread isolation (two users back-to-back on one worker).

The single-account golden regression lives separately:

```sh
cargo test -p sweat_jar --lib replay_account_history
```
