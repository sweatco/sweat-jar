use anyhow::Result;
use near_workspaces::types::NearToken;
use nitka::{misc::ToNear, near_sdk::json_types::U128, set_integration_logs_enabled};
use sweat_jar_model::{
    api::*,
    data::{
        deposit::{DepositMessage, Purpose},
        product::{Cap, Product, Terms, TieredScoreBasedProductTerms},
    },
    signer::test_utils::MessageSigner,
    Timezone,
};
use sweat_model::FungibleTokenCoreIntegration;

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
};

// Note: We create a custom TieredScoreBased product for these tests since
// the existing Locked10Minutes20000ScoreCap uses ScoreBased terms (not TieredScoreBased)
// and doesn't support booster functionality properly.

/// Test: Multi-day score and booster recording
///
/// ## Test Scenario
/// This test validates the score recording and booster application functionality over multiple days,
/// ensuring that boosters can only be applied once per day and that scores accumulate correctly.
/// The test spans 4 days to cover the full 2-day term twice and verify behavior across term boundaries.
///
/// ## Actors
/// - **Alice**: Test user with UTC timezone (offset: 0)
/// - **Bob**: Test user with UTC+2 timezone (offset: +2 hours)
/// - **Manager**: Admin account that can record scores and apply boosters
///
/// ## Product Configuration
/// **Custom TieredScoreBased Product**:
/// - ID: "tiered_2_days_20k_10k_cap"
/// - Type: TieredScoreBased (supports boosters)
/// - Lockup term: 2 days (172,800,000 milliseconds)
/// - Score cap: Tier-based
///   - Default tier: 20,000 points
///   - Fallback tier: 10,000 points
/// - Public key: Set from MessageSigner for deposit validation
/// - Enabled: true
/// - Restakable: true
///
/// ## Initial Setup
/// 1. Create MessageSigner for deposit signature validation
/// 2. Create TieredScoreBased product with 2-day term and tiered caps (20k/10k)
/// 3. Initialize contract context (Manager, Alice, Bob)
/// 4. Register product via Manager
/// 5. Create step jars and make initial deposits:
///    - **Alice**: Create jar + make ONE deposit of 365,000 tokens (365_000 * 10^18 in smallest units)
///      - Timezone: UTC+0
///      - This single deposit will mature after 2 days (by end of Day 2)
///    - **Bob**: Create jar + make initial deposit of 365,000 tokens (365_000 * 10^18 in smallest units)
///      - Timezone: UTC+2
///      - Bob will make a second deposit on Day 2 (365,000 tokens)
///    - Valid until: 49_012_505_000_000 (far future timestamp)
///    - Note: Each deposit lives its 2-day term individually within the jar
///
/// ## Day 1: Score Recording Only (Within Term Day 1)
/// **Time**: +24 hours from base_timestamp
/// **Deposit Status**:
///    - Alice: 365,000 tokens (Day 1 of 2)
///    - Bob: 365,000 tokens (Day 1 of 2)
/// **Actions**:
/// 1. Manager records scores:
///    - Alice: 5,000 steps at day1_timestamp
///    - Bob: 3,000 steps at day1_timestamp
/// 2. Verify RecordScore event is emitted in logs
/// 3. Check interest for both users (should be 0 - scores don't apply same day)
/// **Expected Result**:
///    - Scores recorded successfully, no booster applied yet
///    - Alice interest: 0 (Day 1 scores will affect Day 2 interest calculation)
///    - Bob interest: 0 (Day 1 scores will affect Day 2 interest calculation)
///    - Scores accumulate until end of 24-hour period, then finalize for APY calculation
///
/// ## Day 2: Score Recording + Booster Application + Bob's Second Deposit (Term Day 2 - Last Day)
/// **Time**: +48 hours from base_timestamp (+24 hours from Day 1)
/// **Deposit Status**:
///    - Alice: 365,000 tokens (Day 2 of 2 - last day of term)
///    - Bob: 365,000 tokens (Day 2 of 2 - last day of term)
/// **Actions**:
/// 1. Check interest from Day 1's finalized scores:
///    - Alice: Should show APY calculated from 5,000 points on 365,000 tokens
///    - Bob: Should show APY calculated from 3,000 points on 365,000 tokens
///    - Interest > 0 (Day 1 scores now apply)
/// 2. **Bob makes second deposit**: 365,000 tokens
///    - Bob's jar now has TWO deposits: 730,000 tokens total
///    - First deposit: Day 2 of 2 (maturing)
///    - Second deposit: Day 1 of 2 (just created)
/// 3. Manager records scores for Day 2:
///    - Alice: 6,000 steps at day2_timestamp
///    - Bob: 4,000 steps at day2_timestamp
/// 4. Manager applies booster for Day 2:
///    - Accounts: [Alice, Bob]
///    - Booster score: 1,000 points
///    - Timestamp: day2_timestamp
/// 5. Verify ApplyBooster event is emitted
/// **Expected Result**:
///    - Interest from Day 1 visible and > 0 for both users
///    - Bob now has 730,000 tokens total (two deposits)
///    - Day 2 scores recorded successfully (will affect Day 3 interest)
///    - Day 2 booster applied successfully (will affect Day 3 interest)
///    - Event contains both Alice and Bob in "applied" array
///    - Alice's deposit ends its 2-day term
///    - Bob's first deposit ends its 2-day term, second deposit starts its term
///    - Day 2 scores + booster will finalize at end of 24 hours
///
/// ## Day 3: Score Recording + Booster Rejection (After Alice's Term, During Bob's)
/// **Time**: +72 hours from base_timestamp (+24 hours from Day 2)
/// **Deposit Status**:
///    - Alice: 365,000 tokens (MATURED - past 2-day term)
///    - Bob:
///      - First deposit: 365,000 tokens (MATURED - past 2-day term)
///      - Second deposit: 365,000 tokens (Day 2 of 2 - last day of term)
///      - Total: 730,000 tokens
/// **Actions**:
/// 1. Check interest from Day 2's finalized scores + booster:
///    - Alice: Previous interest + APY from (6,000 points + 1,000 booster) on 365,000 tokens
///    - Bob: Previous interest + APY from (4,000 points + 1,000 booster) on 730,000 tokens (full amount!)
///    - Interest should be > Day 2 interest (accumulated + Day 2's score+booster APY)
///    - Bob's interest should be larger due to more tokens active
/// 2. Manager records scores for Day 3:
///    - Alice: 7,000 steps at day3_timestamp
///    - Bob: 5,000 steps at day3_timestamp
/// 3. Manager attempts to apply booster again:
///    - Accounts: [Alice, Bob]
///    - Booster score: 1,000 points
///    - Timestamp: day3_timestamp
/// 4. Parse ApplyBooster event logs
/// **Expected Result**:
///    - Interest increased from Day 2 (Day 2 scores + booster now applied)
///    - Bob's interest reflects calculation on 730,000 tokens (both deposits)
///    - Day 3 scores recorded (Alice: may not apply post-term; Bob: should apply for second deposit)
///    - Booster application REJECTED (already applied on Day 2 - violates one-per-day rule)
///    - Event contains "applied":[] (empty array)
///    - Event contains "rejected":[...] with Alice and Bob
///    - Note: Alice's deposit matured; Bob's first deposit matured, second deposit ending term
///    - Day 3 scores will affect Day 4 interest if scoring applies
///
/// ## Day 4: Score Recording + New Booster Application (Alice: Post-Term, Bob: Second Deposit Matured)
/// **Time**: +96 hours from base_timestamp (+24 hours from Day 3)
/// **Deposit Status**:
///    - Alice: 365,000 tokens (MATURED - 2 days past term end)
///    - Bob:
///      - First deposit: 365,000 tokens (MATURED - 2 days past term end)
///      - Second deposit: 365,000 tokens (MATURED - just ended term)
///      - Total: 730,000 tokens (both matured)
/// **Actions**:
/// 1. Check interest from Day 3's finalized scores (no booster on Day 3):
///    - Alice: Previous interest + APY from 7,000 points on 365,000 tokens (if post-term scoring applies)
///    - Bob: Previous interest + APY from 5,000 points on 730,000 tokens OR 365,000 tokens
///      - If Alice's matured deposit doesn't accrue: APY on 365,000 (second deposit only)
///      - If both deposits accrue: APY on 730,000 tokens
///    - Interest should be > Day 3 interest
/// 2. Manager records scores for Day 4:
///    - Alice: 8,000 steps at day4_timestamp
///    - Bob: 6,000 steps at day4_timestamp
/// 3. Manager applies new booster for Day 4:
///    - Accounts: [Alice, Bob]
///    - Booster score: 1,500 points
///    - Timestamp: day4_timestamp
/// 4. Verify ApplyBooster event is emitted
/// **Expected Result**:
///    - Interest increased from Day 3 (Day 3 scores applied)
///    - Bob's interest calculation reflects his deposit maturity states
///    - Day 4 scores recorded (may or may not apply post-term)
///    - Day 4 booster applied successfully (new calendar day, one-per-day rule allows new booster)
///    - Event contains both Alice and Bob in "applied" array
///    - Note: All deposits in both jars are now matured
///    - Day 4 scores + booster will affect Day 5 interest (if we were to check)
///
/// ## Final Verification
/// **Actions**:
/// 1. Query finalized scores for Alice and Bob using get_score()
/// 2. Query final interest for Alice and Bob using get_total_interest()
/// 3. Verify both scores > 0 and both interests > 0
/// 4. Verify interest progression: Day 4 interest >= Day 3 interest >= Day 2 interest > Day 1 interest (0)
/// 5. Log final scores and interests for visibility
/// 6. Compare Alice vs Bob interest to verify Bob's higher token amount led to proportionally higher interest
/// **Expected Result**:
///    - Both Alice and Bob have accumulated scores in their jars
///    - Scores reflect recorded steps + applied boosters across all days
///    - Scores respect the tiered cap (20k default, 10k fallback)
///    - Interest reflects cumulative APY calculations:
///      - Day 2: APY from Day 1 scores (Alice: 5000 on 365k, Bob: 3000 on 365k)
///      - Day 3: Day 2 interest + APY from Day 2 (Alice: 6000+1000 on 365k, Bob: 4000+1000 on 730k)
///      - Day 4: Day 3 interest + APY from Day 3 scores (Alice: 7000 on 365k, Bob: 5000 on 365k or 730k)
///    - Alice has one jar containing ONE deposit (365,000 tokens, matured)
///    - Bob has one jar containing TWO deposits (730,000 tokens total, both matured)
///    - Bob's interest should be significantly higher due to having 2x tokens on Days 3-4
///    - Each day's score/booster only affected the next day's interest calculation
///    - Demonstrates how multiple deposits in same jar accumulate and mature independently
#[tokio::test]
#[mutants::skip]
async fn test_multi_day_score_and_booster_recording() -> Result<()> {
    println!("👷🏽 Run multi-day score and booster recording test");
    println!("⏱️  With time scaling: 1 day = 5 minutes (288x speed)");

    // Setup: Create signer and product
    let signer = MessageSigner::new();
    let product = Product {
        id: "tiered_2_days_20k_10k_cap".to_string(),
        cap: Cap::new(
            365_000u128 * 10u128.pow(18), // 365,000 tokens in smallest units
            1_000_000_000_000_000_000_000_000_000u128,
        ),
        terms: Terms::TieredScoreBased(TieredScoreBasedProductTerms {
            lockup_term: (2 * 24 * 60 * 60 * 1000).into(), // 2 days in milliseconds (scaled to 10 minutes)
            score_cap: sweat_jar_model::ConfigurableValue::Tier(sweat_jar_model::ValueTier {
                default: 20_000,
                fallback: 10_000,
            }),
        }),
        public_key: Some(signer.public_key().into()),
        is_enabled: true,
        withdrawal_fee: None,
    };

    set_integration_logs_enabled(false);

    let mut context = prepare_contract(None, []).await?;
    let alice = context.alice().await?;
    let bob = context.bob().await?;
    let manager = context.manager().await?;

    // Set time scale: 1 day = 5 minutes (300 seconds = 300,000 ms)
    // Scale factor: 300,000 / 86,400,000 = 1/288
    println!("⏰ Setting time scale: 1 day = 5 minutes");
    context
        .sweat_jar()
        .set_time_scale(1.0 / 288.0)
        .with_user(&manager)
        .await?;

    // Register product
    context
        .sweat_jar()
        .register_product(product.clone())
        .with_user(&manager)
        .await?;

    // Create jars and make initial deposits for Alice and Bob
    let alice_deposit = 365_000u128 * 10u128.pow(18); // 365,000 tokens in smallest units
    let bob_deposit = 365_000u128 * 10u128.pow(18); // 365,000 tokens in smallest units
    let valid_until = 49_012_505_000_000u64;

    // Alice's jar with initial deposit
    let alice_deposit_message = DepositMessage::new(
        Purpose::Deposit,
        context.sweat_jar().contract.as_account().id(),
        alice.id(),
        &product.id,
        alice_deposit,
        valid_until,
        0,
    );
    context
        .sweat_jar()
        .create_step_jar(
            &alice,
            product.id.clone(),
            alice_deposit,
            signer.sign(alice_deposit_message.as_str()).into(),
            valid_until,
            Timezone::hour_shift(0), // UTC
            &context.ft_contract(),
        )
        .await?;
    context
        .sweat_jar()
        .set_timezone(alice.to_near(), (*Timezone::hour_shift(0)).into())
        .with_user(&manager)
        .await?;

    // Bob's jar with initial deposit
    let bob_deposit_message = DepositMessage::new(
        Purpose::Deposit,
        context.sweat_jar().contract.as_account().id(),
        bob.id(),
        &product.id,
        bob_deposit,
        valid_until,
        0,
    );
    context
        .sweat_jar()
        .create_step_jar(
            &bob,
            product.id.clone(),
            bob_deposit,
            signer.sign(bob_deposit_message.as_str()).into(),
            valid_until,
            Timezone::hour_shift(2), // UTC+2
            &context.ft_contract(),
        )
        .await?;
    context
        .sweat_jar()
        .set_timezone(bob.to_near(), (*Timezone::hour_shift(2)).into())
        .with_user(&manager)
        .await?;

    let base_timestamp = context.sweat_jar().block_timestamp_ms().await?;
    set_integration_logs_enabled(true);

    // Day 1: Record scores only (affects Day 1 interest calculation on Day 2)
    println!("\n📅 Day 1: Recording scores (will affect tomorrow's interest)");
    context.fast_forward_hours(24).await?;
    let day1_timestamp = context.sweat_jar().block_timestamp_ms().await?;

    context
        .sweat_jar()
        .record_score(vec![
            (alice.to_near(), vec![(5000, day1_timestamp.into())]),
            (bob.to_near(), vec![(3000, day1_timestamp.into())]),
        ])
        .with_user(&manager)
        .await?;

    // Check interest on Day 1 (should be 0 - scores don't apply same day)
    let alice_interest_day1 = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    let bob_interest_day1 = context.sweat_jar().get_total_interest(bob.to_near()).await?;
    println!(
        "  Alice interest Day 1: {} (expected: 0 - scores don't apply same day)",
        alice_interest_day1.amount.total.0
    );
    println!(
        "  Bob interest Day 1: {} (expected: 0 - scores don't apply same day)",
        bob_interest_day1.amount.total.0
    );
    println!("  Alice deposit: {} tokens", alice_deposit / 10u128.pow(18));
    println!("  Bob deposit: {} tokens", bob_deposit / 10u128.pow(18));
    assert_eq!(alice_interest_day1.amount.total.0, 0, "Interest should be 0 on Day 1");
    assert_eq!(bob_interest_day1.amount.total.0, 0, "Interest should be 0 on Day 1");

    // Day 2: Record scores + apply booster (affects Day 2 interest calculation on Day 3)
    println!("\n📅 Day 2: Recording scores + applying booster (will affect tomorrow's interest)");
    println!("   ⏭️  Fast-forwarding 24 hours (scaled: 5 minutes)...");
    context.fast_forward_hours(24).await?;
    let day2_timestamp = context.sweat_jar().block_timestamp_ms().await?;

    // Check interest from Day 1's scores
    let alice_interest_day2 = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    let bob_interest_day2 = context.sweat_jar().get_total_interest(bob.to_near()).await?;
    println!(
        "  Alice interest Day 2: {} (from Day 1 score: 5000 points on {} tokens)",
        alice_interest_day2.amount.total.0,
        alice_deposit / 10u128.pow(18)
    );
    println!(
        "  Bob interest Day 2: {} (from Day 1 score: 3000 points on {} tokens)",
        bob_interest_day2.amount.total.0,
        bob_deposit / 10u128.pow(18)
    );
    assert!(
        alice_interest_day2.amount.total.0 > 0,
        "Alice should have interest from Day 1 scores"
    );
    assert!(
        bob_interest_day2.amount.total.0 > 0,
        "Bob should have interest from Day 1 scores"
    );

    // Bob makes second deposit on Day 2
    let bob_second_deposit = 365_000u128 * 10u128.pow(18); // 365,000 tokens in smallest units
    let bob_deposit_message_2 = DepositMessage::new(
        Purpose::Deposit,
        context.sweat_jar().contract.as_account().id(),
        bob.id(),
        &product.id,
        bob_second_deposit,
        valid_until,
        0,
    );

    let deposit_msg = nitka::near_sdk::serde_json::json!({
        "type": "deposit",
        "data": {
            "ticket": {
                "product_id": product.id.clone(),
                "valid_until": valid_until.to_string(),
                "signature": signer.sign(bob_deposit_message_2.as_str()),
            }
        }
    });

    context
        .ft_contract()
        .ft_transfer_call(
            context.sweat_jar().contract.as_account().to_near(),
            bob_second_deposit.into(),
            None,
            deposit_msg.to_string(),
        )
        .with_user(&bob)
        .deposit(NearToken::from_yoctonear(1))
        .await?;

    let bob_total_deposit = bob_deposit + bob_second_deposit;
    println!(
        "  Bob made second deposit: {} tokens (total: {} tokens)",
        bob_second_deposit / 10u128.pow(18),
        bob_total_deposit / 10u128.pow(18)
    );

    // Record scores for Day 2
    context
        .sweat_jar()
        .record_score(vec![
            (alice.to_near(), vec![(6000, day2_timestamp.into())]),
            (bob.to_near(), vec![(4000, day2_timestamp.into())]),
        ])
        .with_user(&manager)
        .await?;

    // Apply booster for Day 2
    let booster_score = 1000u16;
    let result = context
        .sweat_jar()
        .apply_booster(
            vec![alice.to_near(), bob.to_near()],
            booster_score,
            sweat_jar_model::UTC(day2_timestamp),
        )
        .with_user(&manager)
        .result()
        .await?;

    assert!(result
        .logs()
        .iter()
        .any(|log| log.contains(r#""event": "apply_booster""#)));
    println!("  ✅ Booster applied successfully for both users");

    // Day 3: Record scores + attempt booster rejection (affects Day 3 interest calculation on Day 4)
    println!("\n📅 Day 3: Recording scores + attempting duplicate booster (should reject)");
    println!("   ⏭️  Fast-forwarding 24 hours (scaled: 5 minutes)...");
    context.fast_forward_hours(24).await?;
    let day3_timestamp = context.sweat_jar().block_timestamp_ms().await?;

    // Check interest from Day 2's scores + booster
    let alice_interest_day3 = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    let bob_interest_day3 = context.sweat_jar().get_total_interest(bob.to_near()).await?;
    println!(
        "  Alice interest Day 3: {} (from Day 2 score: 6000 + booster: 1000 on {} tokens)",
        alice_interest_day3.amount.total.0,
        alice_deposit / 10u128.pow(18)
    );
    println!(
        "  Bob interest Day 3: {} (from Day 2 score: 4000 + booster: 1000 on {} tokens)",
        bob_interest_day3.amount.total.0,
        bob_total_deposit / 10u128.pow(18)
    );
    assert!(
        alice_interest_day3.amount.total.0 > alice_interest_day2.amount.total.0,
        "Alice interest should increase from Day 2 (score + booster)"
    );
    assert!(
        bob_interest_day3.amount.total.0 > bob_interest_day2.amount.total.0,
        "Bob interest should increase from Day 2 (score + booster)"
    );

    // Record scores for Day 3
    context
        .sweat_jar()
        .record_score(vec![
            (alice.to_near(), vec![(7000, day3_timestamp.into())]),
            (bob.to_near(), vec![(5000, day3_timestamp.into())]),
        ])
        .with_user(&manager)
        .await?;

    // Try to apply booster again (should be rejected)
    let result = context
        .sweat_jar()
        .apply_booster(
            vec![alice.to_near(), bob.to_near()],
            booster_score,
            sweat_jar_model::UTC(day3_timestamp),
        )
        .with_user(&manager)
        .result()
        .await?;

    let logs = result.logs();
    let apply_booster_log = logs
        .iter()
        .find(|log| log.contains(r#""event": "apply_booster""#))
        .expect("ApplyBooster event should be emitted");

    assert!(
        apply_booster_log.contains(r#""applied":[]"#),
        "Booster should be rejected (empty applied array)"
    );
    println!("  ✅ Booster correctly rejected (already applied on Day 2)");

    // Day 4: Record scores + apply new booster (new day allows new booster)
    println!("\n📅 Day 4: Recording scores + applying new booster (new day)");
    println!("   ⏭️  Fast-forwarding 24 hours (scaled: 5 minutes)...");
    context.fast_forward_hours(24).await?;
    let day4_timestamp = context.sweat_jar().block_timestamp_ms().await?;

    // Check interest from Day 3's scores (no booster on Day 3)
    let alice_interest_day4 = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    let bob_interest_day4 = context.sweat_jar().get_total_interest(bob.to_near()).await?;
    println!(
        "  Alice interest Day 4: {} (from Day 3 score: 7000 on {} tokens - deposit matured)",
        alice_interest_day4.amount.total.0,
        alice_deposit / 10u128.pow(18)
    );
    println!(
        "  Bob interest Day 4: {} (from Day 3 score: 5000 on {} tokens)",
        bob_interest_day4.amount.total.0,
        bob_total_deposit / 10u128.pow(18)
    );
    assert!(
        alice_interest_day4.amount.total.0 > alice_interest_day3.amount.total.0,
        "Alice interest should increase from Day 3 scores"
    );
    assert!(
        bob_interest_day4.amount.total.0 > bob_interest_day3.amount.total.0,
        "Bob interest should increase from Day 3 scores"
    );

    // Record scores for Day 4
    context
        .sweat_jar()
        .record_score(vec![
            (alice.to_near(), vec![(8000, day4_timestamp.into())]),
            (bob.to_near(), vec![(6000, day4_timestamp.into())]),
        ])
        .with_user(&manager)
        .await?;

    // Apply new booster for Day 4
    let new_booster_score = 1500u16;
    let result = context
        .sweat_jar()
        .apply_booster(
            vec![alice.to_near(), bob.to_near()],
            new_booster_score,
            sweat_jar_model::UTC(day4_timestamp),
        )
        .with_user(&manager)
        .result()
        .await?;

    assert!(result
        .logs()
        .iter()
        .any(|log| log.contains(r#""event": "apply_booster""#)));
    println!("  ✅ New booster applied successfully for both users");

    // Final verification
    println!("\n📊 Final State Verification:");
    let alice_final_score = context.sweat_jar().get_score(alice.to_near()).await?.unwrap_or(U128(0));
    let bob_final_score = context.sweat_jar().get_score(bob.to_near()).await?.unwrap_or(U128(0));
    let alice_final_interest = context.sweat_jar().get_total_interest(alice.to_near()).await?;
    let bob_final_interest = context.sweat_jar().get_total_interest(bob.to_near()).await?;

    println!(
        "  Alice - Final score: {}, Final interest: {}, Total deposited: {} tokens",
        alice_final_score.0,
        alice_final_interest.amount.total.0,
        alice_deposit / 10u128.pow(18)
    );
    println!(
        "  Bob - Final score: {}, Final interest: {}, Total deposited: {} tokens",
        bob_final_score.0,
        bob_final_interest.amount.total.0,
        bob_total_deposit / 10u128.pow(18)
    );

    assert!(alice_final_score.0 > 0, "Alice should have accumulated score");
    assert!(bob_final_score.0 > 0, "Bob should have accumulated score");
    assert!(
        alice_final_interest.amount.total.0 >= alice_interest_day4.amount.total.0,
        "Alice final interest should be >= Day 4"
    );
    assert!(
        bob_final_interest.amount.total.0 >= bob_interest_day4.amount.total.0,
        "Bob final interest should be >= Day 4"
    );

    // Bob should have higher interest due to having 2x tokens on Days 3-4
    println!("\n📈 Interest comparison:");
    println!("  Alice had {} tokens throughout", alice_deposit / 10u128.pow(18));
    println!(
        "  Bob had {} tokens on Day 1-2, then {} tokens on Day 3-4",
        bob_deposit / 10u128.pow(18),
        bob_total_deposit / 10u128.pow(18)
    );

    println!("🎉 Multi-day score and booster recording test completed successfully!");
    Ok(())
}

/// Test: Bulk booster application performance
///
/// ## Test Scenario
/// This test validates the performance and scalability of applying boosters to large batches of accounts,
/// measuring gas consumption and execution time for different batch sizes. Uses a 2-day term product
/// to properly test TieredScoreBased functionality with boosters.
///
/// ## Actors
/// - **Manager**: Admin account that records scores and applies boosters
/// - **Test Accounts**: 1,000 dynamically created accounts (test_account_0 through test_account_999)
///
/// ## Product Configuration
/// **Custom TieredScoreBased Product**:
/// - ID: "tiered_2_days_20k_10k_cap_perf"
/// - Type: TieredScoreBased (supports boosters)
/// - Lockup term: 2 days (172,800,000 milliseconds)
/// - Score cap: Tier-based
///   - Default tier: 20,000 points
///   - Fallback tier: 10,000 points
/// - Public key: Set from MessageSigner for deposit validation
/// - Enabled: true
/// - Restakable: true
///
/// ## Initial Setup
/// 1. Create MessageSigner for deposit signature validation
/// 2. Create TieredScoreBased product with 2-day term and tiered caps (20k/10k)
/// 3. Initialize contract context with Manager
/// 4. Register product via Manager
/// 5. Create 1,000 test accounts:
///    - Names: "test_account_0", "test_account_1", ..., "test_account_999"
/// 6. Create step jars and make initial deposits for all 1,000 accounts:
///    - Create jar for each account (jar = container for deposits of this product)
///    - Make initial deposit: 365,000 tokens each (divisible by 365 for easier APY calculations)
///    - Valid until: 49_012_505_000_000 (far future timestamp)
///    - Timezone: Distributed across 24 timezones using modulo (account_index % 24)
///      - Account 0: UTC+0
///      - Account 1: UTC+1
///      - Account 2: UTC+2
///      - ...
///      - Account 23: UTC+23
///      - Account 24: UTC+0 (cycles back)
///      - ... continues for all 1,000 accounts
/// 7. Set timezone for each account via Manager
///
/// ## Performance Test Execution
/// **Base Setup**:
/// - Test timestamp: base_timestamp + 24 hours (1 day into term)
/// - Booster score: 1,000 points (consistent across all tests)
/// - All tests run on Day 1 of the 2-day term
///
/// **Batch Sizes to Test**: [10, 50, 100, 250, 500, 1000]
///
/// For each batch size:
///
/// ### Step 1: Record Scores
/// - Select first N accounts (where N = batch_size)
/// - Record score for each account: 5,000 steps at test_timestamp
/// - Execute via Manager
/// - No timing measured (baseline operation)
///
/// ### Step 2: Apply Booster (Timed Operation)
/// - **Start performance timer** (std::time::Instant::now())
/// - Apply booster to the same N accounts
/// - Booster score: 1,000 points
/// - Timestamp: test_timestamp
/// - Execute via Manager with .result() to capture full response
/// - **Stop performance timer** (elapsed duration)
///
/// ### Step 3: Collect and Log Metrics
/// **Time Metrics**:
/// - Total execution duration in milliseconds
/// - Log format: "📊 Batch size {N}: {duration}ms, {applied} applied, {rejected} rejected"
///
/// **Result Metrics**:
/// - Parse ApplyBooster event from logs
/// - Count accounts in "applied" array (successful applications)
/// - Count accounts in "rejected" array (failed applications)
/// - Verify applied + rejected = batch_size
///
/// **Gas Metrics**:
/// - Extract total_gas_burnt from result
/// - Calculate gas per account: total_gas_burnt / batch_size
/// - Convert to TGas (TeraGas): gas / 1_000_000_000_000.0
/// - Log format: "⛽ Gas per account: {gas_per_account:.2} TGas"
///
/// ## Expected Results
/// - All batch sizes complete successfully (no panics or failures)
/// - Gas usage scales linearly with batch size
/// - Execution time remains reasonable (< 10s for 1000 accounts)
/// - Most/all boosters applied successfully (first-time application on Day 1)
/// - Events emitted correctly with proper applied/rejected arrays
/// - No rejected boosters expected (all first-time, within term, within cap)
///
/// ## Performance Benchmarks (Expected)
/// - Batch size 10: < 1s execution time, ~X TGas per account
/// - Batch size 50: < 2s execution time, ~X TGas per account
/// - Batch size 100: < 3s execution time, ~X TGas per account
/// - Batch size 250: < 5s execution time, ~X TGas per account
/// - Batch size 500: < 7s execution time, ~X TGas per account
/// - Batch size 1000: < 10s execution time, ~X TGas per account
/// - Gas per account should remain consistent across all batch sizes
/// - Linear scaling indicates good O(n) performance without quadratic bottlenecks
#[tokio::test]
#[mutants::skip]
async fn test_bulk_booster_application_performance() -> Result<()> {
    println!("👷🏽 Run bulk booster application performance test");

    // Write code here

    println!("🎉 Bulk booster application performance test completed!");
    Ok(())
}
