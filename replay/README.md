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

Three commands, in order. Use `--release` — the `step_packages` ingest is ~285M
rows and the `run` loop replays the contract once per account.

```sh
# 1. Refresh the on-chain product catalogue (committed to the repo).
#    Calls get_products() on v2.jars.sweat via mainnet JSON RPC.
cargo run -p replay --release -- fetch-products --out test_data/products.json

# 2. Ingest the 4 source CSVs (+ optional snapshots.ndjson) into SQLite.
cargo run -p replay --release -- build-db --db replay.db --test-data-dir test_data

# 3. Reconcile and write the report.
cargo run -p replay --release -- run --db replay.db --out reconciliation.csv --products test_data/products.json
```

For dev iteration, restrict the build and run to a handful of accounts (see the
flags below): `build-db --sample 50` then `run --sample 50`.

## `build-db` flags

| flag | default | meaning |
|------|---------|---------|
| `--db <path>` | — | SQLite file to create / append to (required) |
| `--test-data-dir <dir>` | `test_data` | directory holding the source CSVs |
| `--only <t1,t2,...>` | all | ingest only these tables: `users`, `jar_events`, `step_packages`, `subscriptions`, `snapshots` (an unknown name is rejected) |
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

## Input CSVs (`test_data/`)

| file | on-chain / notes | key column |
|------|------------------|-----------|
| `users.csv` | `near_account_id` is the on-chain `AccountId`; `sweatcoin_user_id` is not ingested | `account_id` |
| `jar_events.csv` | `event_type` ∈ `deposit`/`claim`/`withdraw`/`restake`/`merge`; `near_block_timestamp` is ISO-8601 ms `Z`; `amount` is the token base unit as a decimal string | `account_id` |
| `step_packages.csv` | `created_at` is `YYYY-MM-DD HH:MM:SS UTC`; `steps` clamped to `65535` at ingest | `account_id` |
| `max_subscriptions.csv` | `action_type` ∈ `subscribed`/`expired` → `active` `1`/`0` | `user_id` (== `account_id`) |

`account_id` is the same integer across all four files. Optional
**`snapshots.ndjson`** — one JSON object per line with `near_account_id`,
`account_state`, and `products_referenced` (mirrors
`test_data/account_full_state_190375496.json`). **Not produced yet** — see
Limitations.

## SQLite schema

Six tables (verbatim from `replay/src/db/schema.rs`):

```
users         (account_id PK, near_account_id)
jar_events    (account_id, ts_ms, seq, event_type, product_id, amount)
step_packages (account_id, ts_ms, steps)
subscriptions (account_id, ts_ms, active)          -- subscribed=1, expired=0
snapshots     (account_id PK, state_json)          -- verbatim ndjson line
meta          (key PK, value)
```

`meta` rows: `window_h_ms`, `window_t_end_ms`, `built_at` (build time, epoch ms).

Notes:
- Indexes (`ix_jar_events_acct`, `ix_step_packages_acct`,
  `ix_subscriptions_acct`) are created **after** the bulk load.
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

- **No baseline snapshots yet.** `snapshots.ndjson` needs an archival-RPC
  extraction of every account's state at block `H`, which is not built —
  `ArchivalRpcSnapshotSource` in `snapshot.rs` is a stub that always errors.
  Until it exists, any account that held jars before `H` replays from an empty
  account and shows a large negative `delta` with `status = no_baseline` (or
  `error:` if the empty-account replay panics). **Only accounts that first
  appear after `H` are trustworthy today.**
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
