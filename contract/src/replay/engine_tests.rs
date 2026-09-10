use sweat_jar_model::data::product::{Apy, Cap, FixedProductTerms, Product, Terms};
use sweat_jar_primitives::UDecimal;

use super::engine::{run_timeline, Action, Baseline, Event, ReplayStatus, Timeline};

fn fixed_product() -> Product {
    Product {
        id: "365d_12apy".to_string(),
        cap: Cap::new(1_000_000_000_000_000_000, 500_000 * 10u128.pow(24)),
        terms: Terms::Fixed(FixedProductTerms {
            lockup_term: 31_536_000_000u64.into(),
            apy: Apy::Constant(UDecimal::new(12, 2)),
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    }
}

#[test]
fn deposit_then_claim_after_a_year_yields_roughly_apy() {
    let account_id: near_sdk::AccountId = "acc.near".parse().unwrap();
    let one_year_ms = 31_536_000_000u64;
    let deposit_amount = 1_000 * 10u128.pow(18);

    let timeline = Timeline {
        events: vec![
            Event {
                ts_ms: 10,
                seq: 0,
                action: Action::Deposit {
                    product_id: "365d_12apy".into(),
                    amount: deposit_amount,
                },
            },
            Event {
                ts_ms: one_year_ms + 100,
                seq: 1,
                action: Action::Claim,
            },
        ],
    }
    .sorted();

    let outcome = run_timeline(Baseline { account_id, raw_account: None, timezone_ms: None }, &[fixed_product()], 0, timeline);

    assert!(matches!(outcome.status, ReplayStatus::Ok));
    assert_eq!(outcome.per_claim.len(), 1);
    // 12% of 1000 SWEAT, within 1%.
    let expected = 120 * 10u128.pow(18);
    let claimed = outcome.total_claimed;
    assert!(claimed.abs_diff(expected) < expected / 100, "claimed {claimed} vs {expected}");
}

#[test]
fn claim_with_no_jars_is_reported_not_panicked() {
    let account_id: near_sdk::AccountId = "empty.near".parse().unwrap();
    let timeline = Timeline {
        events: vec![Event {
            ts_ms: 5,
            seq: 0,
            action: Action::Claim,
        }],
    }
    .sorted();
    let outcome = run_timeline(Baseline { account_id, raw_account: None, timezone_ms: None }, &[fixed_product()], 0, timeline);
    // Either Ok with zero claims, or Error — never a process panic.
    match outcome.status {
        ReplayStatus::Ok => assert_eq!(outcome.total_claimed, 0),
        ReplayStatus::Error(_) => {}
    }
}
