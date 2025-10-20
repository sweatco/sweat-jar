use anyhow::{anyhow, Result};
use near_workspaces::{types::AccountId, Account};
use nitka::{
    near_sdk::{json_types::U128, serde_json},
    set_integration_logs_enabled,
};
use std::{str::FromStr, time::Instant};
use sweat_jar_model::{
    api::*,
    data::product::{Cap, Product, Terms, TieredScoreBasedProductTerms},
    signer::test_utils::MessageSigner,
    Timezone,
};

use crate::context::{prepare_contract, Context as TestContext, IntegrationContext};

const ACCOUNT_ID_SEED: [u8; 32] = [
    0xdd, 0xb5, 0xa4, 0x8a, 0xe7, 0x8c, 0x17, 0xca, 0xc1, 0xeb, 0x74, 0xff, 0xd8, 0x73, 0x51, 0x52, 0x30, 0xd9, 0x3f,
    0x97, 0x24, 0x9e, 0x58, 0x68, 0xb7, 0xd5, 0x46, 0xa2, 0xfc, 0x99, 0x01, 0x13,
];
const BOOSTER_SCORE: u16 = 1_000;
const SCORE_VALUE: u16 = 5_000;
const RECORD_BATCH_SIZE: usize = 80;
const APPLY_PROBE_BATCH_SIZE: usize = 85;
const INITIAL_PREPARED_ACCOUNTS: usize = APPLY_PROBE_BATCH_SIZE;
const APPLY_LIMIT_SEARCH_STEP: usize = 5;
const APPLY_LIMIT_SEARCH_MAX: usize = 250;
const APPLY_LIMIT_SEED_OFFSET: usize = 1_000;
const GAS_LIMIT_TGAS: f64 = 300.0;
const LOG_LIMIT_BYTES: usize = 16_384;

fn derive_account_id(index: usize) -> AccountId {
    let mut seed = ACCOUNT_ID_SEED;
    let mut value = (index + 1) as u32;

    for byte in seed.iter_mut().rev() {
        if value == 0 {
            break;
        }

        let (new_byte, carry) = byte.overflowing_add((value & 0xFF) as u8);
        *byte = new_byte;
        value = (value >> 8) + u32::from(carry);
    }

    let hex = seed.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    AccountId::from_str(&hex).expect("valid account id")
}

#[derive(Clone)]
struct SeedSpec {
    account_id: AccountId,
    principal: u128,
    timezone: Timezone,
}

#[derive(Clone)]
struct BatchOutcome {
    size: usize,
    applied: usize,
    rejected: usize,
    gas_tgas: f64,
    log_len: usize,
    duration_ms: u128,
}

impl BatchOutcome {
    fn hits_limits(&self) -> bool {
        self.gas_tgas >= GAS_LIMIT_TGAS || self.log_len >= LOG_LIMIT_BYTES || self.rejected > 0
    }
}

struct BulkBoosterPerformanceHarness<'a> {
    context: &'a mut TestContext,
    manager: Account,
    product_id: String,
    booster_score: u16,
    seed_specs: Vec<SeedSpec>,
    account_ids: Vec<AccountId>,
    deposit_timestamp: u64,
    score_timestamp: u64,
    score_value: u16,
}

impl<'a> BulkBoosterPerformanceHarness<'a> {
    fn new(
        context: &'a mut TestContext,
        manager: Account,
        product_id: String,
        seed_specs: Vec<SeedSpec>,
        booster_score: u16,
        deposit_timestamp: u64,
        score_timestamp: u64,
        score_value: u16,
    ) -> Self {
        let account_ids = seed_specs.iter().map(|spec| spec.account_id.clone()).collect();

        Self {
            context,
            manager,
            product_id,
            booster_score,
            seed_specs,
            account_ids,
            deposit_timestamp,
            score_timestamp,
            score_value,
        }
    }

    async fn seed_accounts_only(&mut self, count: usize) -> Result<()> {
        for batch in self.seed_specs[..count].chunks(200) {
            let payload = batch
                .iter()
                .map(|spec| (spec.account_id.clone(), U128(spec.principal), spec.timezone))
                .collect::<Vec<_>>();

            self.context
                .sweat_jar()
                .seed_accounts(self.product_id.clone(), payload, self.deposit_timestamp)
                .with_user(&self.manager)
                .await?;
        }

        Ok(())
    }

    async fn record_scores(&mut self, count: usize) -> Result<BatchOutcome> {
        let payload = self
            .seed_specs
            .iter()
            .take(count)
            .map(|spec| {
                (
                    spec.account_id.clone(),
                    vec![(self.score_value, sweat_jar_model::UTC(self.score_timestamp))],
                )
            })
            .collect::<Vec<_>>();

        let started = Instant::now();
        let execution = self
            .context
            .sweat_jar()
            .record_score(payload)
            .with_user(&self.manager)
            .result()
            .await?;
        let duration_ms = started.elapsed().as_millis();

        let log_len = execution
            .logs()
            .iter()
            .find(|log| log.contains(r#""event": "record_score""#))
            .map(|log| log.len())
            .unwrap_or_default();

        Ok(BatchOutcome {
            size: count,
            applied: count,
            rejected: 0,
            gas_tgas: execution.total_gas_burnt.as_gas() as f64 / 1_000_000_000_000.0,
            log_len,
            duration_ms,
        })
    }
}

async fn apply_booster_batch(harness: &mut BulkBoosterPerformanceHarness<'_>, count: usize) -> Result<BatchOutcome> {
    let accounts = harness.account_ids.iter().take(count).cloned().collect::<Vec<_>>();

    let started = Instant::now();
    let execution = harness
        .context
        .sweat_jar()
        .apply_booster(
            accounts,
            harness.booster_score,
            sweat_jar_model::UTC(harness.score_timestamp),
        )
        .with_user(&harness.manager)
        .result()
        .await;
    let duration_ms = started.elapsed().as_millis();

    match execution {
        Ok(raw) => {
            let logs_vec: Vec<String> = raw.logs().into_iter().map(|s| s.to_string()).collect();
            let apply_log = logs_vec
                .iter()
                .find(|log| log.contains(r#""event": "apply_booster""#))
                .ok_or_else(|| anyhow!("ApplyBooster event log not found"))?;

            let log_len = apply_log.len();
            let applied = extract_account_count(apply_log, "applied");
            let rejected = extract_account_count(apply_log, "rejected");

            Ok(BatchOutcome {
                size: count,
                applied,
                rejected,
                gas_tgas: raw.total_gas_burnt.as_gas() as f64 / 1_000_000_000_000.0,
                log_len,
                duration_ms,
            })
        }
        Err(err) => Err(anyhow!(err.to_string())),
    }
}

#[tokio::test]
#[mutants::skip]
async fn test_bulk_booster_application_performance() -> Result<()> {
    println!("👷🏽 Run bulk booster application performance test");

    set_integration_logs_enabled(false);

    let signer = MessageSigner::new();
    let product = Product {
        id: "tiered_2_days_20k_10k_cap_perf".to_string(),
        cap: Cap::new(365_000u128 * 10u128.pow(18), 1_000_000_000_000_000_000_000_000_000u128),
        terms: Terms::TieredScoreBased(TieredScoreBasedProductTerms {
            lockup_term: (2 * 24 * 60 * 60 * 1000).into(),
            score_cap: sweat_jar_model::ConfigurableValue::Tier(sweat_jar_model::ValueTier {
                default: 20_000,
                fallback: 10_000,
            }),
        }),
        public_key: Some(signer.public_key().into()),
        is_enabled: true,
        withdrawal_fee: None,
    };

    let mut context = prepare_contract(None, []).await?;
    let manager = context.manager().await?;

    context
        .sweat_jar()
        .set_time_scale(1.0 / 288.0)
        .with_user(&manager)
        .await?;

    context
        .sweat_jar()
        .register_product(product.clone())
        .with_user(&manager)
        .await?;

    let deposit_timestamp = context.sweat_jar().block_timestamp_ms().await?;
    let score_timestamp = deposit_timestamp + 1;

    let seed_specs = (0..INITIAL_PREPARED_ACCOUNTS)
        .map(|index| SeedSpec {
            account_id: derive_account_id(index),
            principal: 365_000u128 * 10u128.pow(18),
            timezone: Timezone::hour_shift(0),
        })
        .collect::<Vec<_>>();

    let mut harness = BulkBoosterPerformanceHarness::new(
        &mut context,
        manager.clone(),
        product.id.clone(),
        seed_specs,
        BOOSTER_SCORE,
        deposit_timestamp,
        score_timestamp,
        SCORE_VALUE,
    );

    harness.seed_accounts_only(INITIAL_PREPARED_ACCOUNTS).await?;

    let record_outcome = harness.record_scores(RECORD_BATCH_SIZE).await?;
    log_outcome("record_score", &record_outcome);

    assert_eq!(
        record_outcome.size, RECORD_BATCH_SIZE,
        "record_score processed unexpected number of accounts"
    );
    assert_eq!(
        record_outcome.applied, RECORD_BATCH_SIZE,
        "expected record_score to apply for all accounts"
    );
    assert_eq!(
        record_outcome.rejected, 0,
        "record_score unexpectedly rejected accounts"
    );
    assert!(
        record_outcome.log_len < LOG_LIMIT_BYTES,
        "record_score log size {} should stay below limit {}",
        record_outcome.log_len,
        LOG_LIMIT_BYTES
    );

    let apply_outcome = apply_booster_batch(&mut harness, APPLY_PROBE_BATCH_SIZE).await?;
    log_outcome("apply_booster", &apply_outcome);

    assert_eq!(
        apply_outcome.size, APPLY_PROBE_BATCH_SIZE,
        "apply_booster processed unexpected number of accounts"
    );
    assert_eq!(
        apply_outcome.applied, APPLY_PROBE_BATCH_SIZE,
        "expected booster to apply for all accounts"
    );
    assert_eq!(
        apply_outcome.rejected, 0,
        "apply_booster unexpectedly rejected accounts"
    );
    assert!(
        apply_outcome.log_len < LOG_LIMIT_BYTES,
        "apply_booster log size {} should stay below limit {}",
        apply_outcome.log_len,
        LOG_LIMIT_BYTES
    );

    drop(harness);

    let mut last_success: Option<BatchOutcome> = None;
    let mut failure: Option<(usize, String)> = None;
    let mut iteration = 0usize;

    for size in (APPLY_PROBE_BATCH_SIZE..=APPLY_LIMIT_SEARCH_MAX).step_by(APPLY_LIMIT_SEARCH_STEP) {
        let start = APPLY_LIMIT_SEED_OFFSET + iteration * APPLY_LIMIT_SEARCH_STEP;
        let seed_specs = (0..size)
            .map(|index| SeedSpec {
                account_id: derive_account_id(start + index),
                principal: 365_000u128 * 10u128.pow(18),
                timezone: Timezone::hour_shift(0),
            })
            .collect::<Vec<_>>();

        let mut probe_harness = BulkBoosterPerformanceHarness::new(
            &mut context,
            manager.clone(),
            product.id.clone(),
            seed_specs,
            BOOSTER_SCORE,
            deposit_timestamp,
            score_timestamp + 1 + iteration as u64,
            SCORE_VALUE,
        );

        probe_harness.seed_accounts_only(size).await?;

        match apply_booster_batch(&mut probe_harness, size).await {
            Ok(outcome) => {
                log_outcome("apply_booster", &outcome);
                assert_eq!(
                    outcome.applied, size,
                    "expected apply_booster to apply for all accounts at batch {}",
                    size
                );
                assert_eq!(
                    outcome.rejected, 0,
                    "apply_booster unexpectedly rejected accounts at batch {}",
                    size
                );
                last_success = Some(outcome);
            }
            Err(err) => {
                failure = Some((size, err.to_string()));
                println!(
                    "🚨 apply_booster failed at {} accounts with runtime message: {}",
                    size, err
                );
                drop(probe_harness);
                break;
            }
        }

        drop(probe_harness);
        iteration += 1;
    }

    let (failure_size, failure_message) = failure.expect("expected to reach apply_booster log limit");
    let last_success = last_success.expect("expected at least one successful apply_booster batch");

    assert!(
        failure_size > last_success.size,
        "apply_booster limit search did not progress to a larger failing batch"
    );
    assert!(
        failure_message.contains("log message"),
        "expected log limit failure, got: {}",
        failure_message
    );
    let headroom = LOG_LIMIT_BYTES.saturating_sub(last_success.log_len);
    println!(
        "ℹ️  Last successful apply_booster batch: {} accounts (log {} bytes, headroom {} bytes)",
        last_success.size, last_success.log_len, headroom
    );
    println!("🚨 apply_booster first failed batch: {} accounts", failure_size);

    println!("🎉 Bulk booster application limit test completed!");
    Ok(())
}

fn log_outcome(label: &str, outcome: &BatchOutcome) {
    println!(
        "📊 [{}] Batch {:5} | applied {:5} | rejected {:3} | gas {:7.2} TGas | log {:5} bytes | duration {:4} ms{}",
        label,
        outcome.size,
        outcome.applied,
        outcome.rejected,
        outcome.gas_tgas,
        outcome.log_len,
        outcome.duration_ms,
        if outcome.hits_limits() { " 🚨" } else { "" }
    );
}

fn extract_account_count(log: &str, key: &str) -> usize {
    let payload = log
        .strip_prefix("EVENT_JSON: ")
        .or_else(|| log.strip_prefix("EVENT_JSON:"))
        .unwrap_or(log);

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
        if let Some(data) = json.get("data") {
            if let Some(array) = data.get(key).and_then(|v| v.as_array()) {
                return array.len();
            }
        }
    }

    0
}
