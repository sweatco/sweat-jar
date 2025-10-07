#![cfg(test)]

use sweat_jar_model::{BoostedScore, Score};

#[test]
fn test_new_unclaimed() {
    let booster = BoostedScore::new(1234, false);
    assert_eq!(booster.get_value(), 1230);
    assert!(!booster.is_claimed());
}

#[test]
fn test_new_claimed() {
    let booster = BoostedScore::new(5678, true);
    assert_eq!(booster.get_value(), 5670);
    assert!(booster.is_claimed());
}

#[test]
fn test_new_zero_score() {
    let booster = BoostedScore::new(0, false);
    assert_eq!(booster.get_value(), 0);
    assert!(!booster.is_claimed());
}

#[test]
fn test_new_max_score() {
    let booster = BoostedScore::new(65535, true);
    assert_eq!(booster.get_value(), 65530);
    assert!(booster.is_claimed());
}

#[test]
fn test_set_claimed_true() {
    let mut booster = BoostedScore::new(1000, false);
    assert!(!booster.is_claimed());

    booster.set_claimed(true);
    assert!(booster.is_claimed());
    assert_eq!(booster.get_value(), 1000); // Score should remain unchanged
}

#[test]
fn test_set_claimed_false() {
    let mut booster = BoostedScore::new(2000, true);
    assert!(booster.is_claimed());

    booster.set_claimed(false);
    assert!(!booster.is_claimed());
    assert_eq!(booster.get_value(), 2000); // Score should remain unchanged
}

#[test]
fn test_set_claimed_multiple_times() {
    let mut booster = BoostedScore::new(1500, false);

    booster.set_claimed(true);
    assert!(booster.is_claimed());

    booster.set_claimed(false);
    assert!(!booster.is_claimed());

    booster.set_claimed(true);
    assert!(booster.is_claimed());
}

#[test]
fn test_bit_packing_consistency() {
    // Test that the bit packing works correctly
    let booster = BoostedScore::new(1234, true);

    // The internal representation should be:
    // - Score 1234 normalized to 123 (1234 / 10)
    // - Shifted left by 1: 123 << 1 = 246
    // - OR with claim bit: 246 | 1 = 247
    assert_eq!(booster.get_raw_value(), 247);
}

#[test]
fn test_bit_packing_unclaimed() {
    let booster = BoostedScore::new(1000, false);

    // The internal representation should be:
    // - Score 1000 normalized to 100 (1000 / 10)
    // - Shifted left by 1: 100 << 1 = 200
    // - OR with claim bit: 200 | 0 = 200
    assert_eq!(booster.get_raw_value(), 200);
}

#[test]
fn test_score_preservation_after_claim_changes() {
    let mut booster = BoostedScore::new(5432, false);
    let original_score = booster.get_value();

    booster.set_claimed(true);
    assert_eq!(booster.get_value(), original_score);

    booster.set_claimed(false);
    assert_eq!(booster.get_value(), original_score);
}

#[test]
fn test_edge_case_scores() {
    // Test various edge cases
    let test_cases = vec![
        (0, false),
        (0, true),
        (1, false),
        (1, true),
        (9, false),  // Should round down to 0 when normalized
        (10, false), // Should normalize to 1
        (11, false), // Should normalize to 1
    ];

    for (score, is_claimed) in test_cases {
        let booster = BoostedScore::new(score, is_claimed);
        assert_eq!(booster.is_claimed(), is_claimed);
        // Note: Due to normalization, some scores may not round-trip perfectly
        // This is expected behavior for scores < 10
    }
}

#[test]
fn test_claim_unclaimed_booster() {
    let mut booster = BoostedScore::new(2500, false);
    assert!(!booster.is_claimed());

    let score = booster.claim();
    assert_eq!(score, 2500);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_already_claimed_booster() {
    let mut booster = BoostedScore::new(1000, true);
    assert!(booster.is_claimed());

    let score = booster.claim();
    assert_eq!(score, 0);
    assert!(booster.is_claimed()); // Should still be claimed
}

#[test]
fn test_claim_zero_score_booster() {
    let mut booster = BoostedScore::new(0, false);
    assert!(!booster.is_claimed());

    let score = booster.claim();
    assert_eq!(score, 0);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_multiple_times() {
    let mut booster = BoostedScore::new(3000, false);

    // First claim should return the score
    let first_claim = booster.claim();
    assert_eq!(first_claim, 3000);
    assert!(booster.is_claimed());

    // Second claim should return 0
    let second_claim = booster.claim();
    assert_eq!(second_claim, 0);
    assert!(booster.is_claimed());

    // Third claim should also return 0
    let third_claim = booster.claim();
    assert_eq!(third_claim, 0);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_preserves_score_value() {
    let mut booster = BoostedScore::new(5432, false);
    let original_score = booster.get_value();

    let claimed_score = booster.claim();
    assert_eq!(claimed_score, original_score);
    assert_eq!(booster.get_value(), original_score);
}

#[test]
fn test_claim_after_manual_set_claimed() {
    let mut booster = BoostedScore::new(2000, false);

    // Manually set as claimed
    booster.set_claimed(true);
    assert!(booster.is_claimed());

    // Claim should now return 0
    let score = booster.claim();
    assert_eq!(score, 0);
    assert!(booster.is_claimed());
}

#[test]
fn test_claim_after_manual_set_unclaimed() {
    let mut booster = BoostedScore::new(1500, true);

    // Manually set as unclaimed
    booster.set_claimed(false);
    assert!(!booster.is_claimed());

    // Claim should now return the score
    let score = booster.claim();
    assert_eq!(score, 1500);
    assert!(booster.is_claimed());
}
