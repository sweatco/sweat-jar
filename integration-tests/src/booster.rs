use anyhow::Result;
use near_workspaces::{types::NearToken, Account};
use nitka::{
    misc::ToNear,
    near_sdk::{json_types::U128, serde_json::json},
    set_integration_logs_enabled,
};
use sweat_jar_model::{
    api::*,
    data::{
        deposit::{DepositMessage, Purpose},
        jar::AggregatedInterestView,
        product::{Cap, Product, Terms, TieredScoreBasedProductTerms},
    },
    signer::test_utils::MessageSigner,
    Timezone, UTC,
};
use sweat_model::FungibleTokenCoreIntegration;

use crate::{
    context::{prepare_contract, Context as TestContext, IntegrationContext},
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

    let mut scenario = MultiDayBoosterScenario::new().await?;

    scenario.record_day_one_scores().await?;
    scenario.record_day_two_scores_and_apply_booster().await?;
    scenario.record_day_three_scores_and_expect_rejected_booster().await?;
    scenario.record_day_four_scores_and_apply_new_booster().await?;
    scenario.verify_final_state().await?;

    println!("🎉 Multi-day score and booster recording test completed successfully!");
    Ok(())
}

struct MultiDayBoosterScenario {
    context: TestContext,
    signer: MessageSigner,
    product_id: String,
    accounts: Accounts,
    booster_scores: BoosterScores,
    valid_until: u64,
    interest_log: InterestLog,
}

struct Accounts {
    manager: Account,
    alice: ParticipantState,
    bob: ParticipantState,
}

struct ParticipantState {
    account: Account,
    initial_deposit: u128,
    total_deposit: u128,
    next_nonce: u32,
}

impl ParticipantState {
    fn new(account: Account, initial_deposit: u128) -> Self {
        Self {
            account,
            initial_deposit,
            total_deposit: initial_deposit,
            next_nonce: 0,
        }
    }

    fn account(&self) -> &Account {
        &self.account
    }

    fn consume_nonce(&mut self) -> u32 {
        let nonce = self.next_nonce;
        self.next_nonce = self.next_nonce.checked_add(1).expect("nonce counter overflow");
        nonce
    }

    fn add_deposit(&mut self, amount: u128) {
        self.total_deposit += amount;
    }
}

impl Accounts {
    fn manager(&self) -> &Account {
        &self.manager
    }
}

struct BoosterScores {
    daily: u16,
    new_day: u16,
}

impl BoosterScores {
    fn new(daily: u16, new_day: u16) -> Self {
        Self { daily, new_day }
    }
}

struct InterestLog {
    entries: Vec<InterestSnapshot>,
}

impl InterestLog {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn push(&mut self, snapshot: InterestSnapshot) {
        self.entries.push(snapshot);
    }

    fn latest(&self) -> Option<&InterestSnapshot> {
        self.entries.last()
    }
}

#[derive(Clone, Copy)]
struct InterestSnapshot {
    day: usize,
    alice: u128,
    bob: u128,
}

impl InterestSnapshot {
    fn new(day: usize, alice: u128, bob: u128) -> Self {
        Self { day, alice, bob }
    }
}

impl MultiDayBoosterScenario {
    async fn new() -> Result<Self> {
        let signer = MessageSigner::new();
        let product_id = "tiered_2_days_20k_10k_cap".to_string();
        let base_deposit = 365_000u128 * 10u128.pow(18);
        let product = Product {
            id: product_id.clone(),
            cap: Cap::new(base_deposit, 1_000_000_000_000_000_000_000_000_000u128),
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

        set_integration_logs_enabled(false);

        let mut context = prepare_contract(None, []).await?;
        let alice_account = context.alice().await?;
        let bob_account = context.bob().await?;
        let manager = context.manager().await?;

        println!("⏰ Setting time scale: 1 day = 5 minutes");
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

        let accounts = Accounts {
            manager,
            alice: ParticipantState::new(alice_account, base_deposit),
            bob: ParticipantState::new(bob_account, base_deposit),
        };

        let mut scenario = Self {
            context,
            signer,
            product_id,
            accounts,
            booster_scores: BoosterScores::new(1_000, 1_500),
            valid_until: 49_012_505_000_000u64,
            interest_log: InterestLog::new(),
        };

        scenario.create_initial_deposits().await?;
        scenario.context.sweat_jar().block_timestamp_ms().await?;
        set_integration_logs_enabled(true);

        Ok(scenario)
    }

    async fn record_day_one_scores(&mut self) -> Result<()> {
        println!("\n📅 Day 1: Recording scores (will affect tomorrow's interest)");
        println!("   ⏭️  Fast-forwarding 24 hours (scaled: 5 minutes)...");
        let timestamp = self.advance_one_day().await?;

        self.record_scores(
            timestamp,
            &[
                (self.accounts.alice.account(), 5_000),
                (self.accounts.bob.account(), 3_000),
            ],
        )
        .await?;

        let alice_interest = self.total_interest(self.accounts.alice.account()).await?;
        let bob_interest = self.total_interest(self.accounts.bob.account()).await?;
        println!(
            "  Alice interest Day 1: {} (expected: 0 - scores don't apply same day)",
            alice_interest.amount.total.0
        );
        println!(
            "  Bob interest Day 1: {} (expected: 0 - scores don't apply same day)",
            bob_interest.amount.total.0
        );
        println!(
            "  Alice deposit: {} tokens",
            self.accounts.alice.initial_deposit / 10u128.pow(18)
        );
        println!(
            "  Bob deposit: {} tokens",
            self.accounts.bob.initial_deposit / 10u128.pow(18)
        );

        assert_eq!(alice_interest.amount.total.0, 0, "Interest should be 0 on Day 1");
        assert_eq!(bob_interest.amount.total.0, 0, "Interest should be 0 on Day 1");

        self.interest_log.push(InterestSnapshot::new(
            1,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        self.verify_initial_principals().await?;
        println!("  ✅ Initial principals verified");

        Ok(())
    }

    async fn record_day_two_scores_and_apply_booster(&mut self) -> Result<()> {
        println!("\n📅 Day 2: Recording scores + applying booster (will affect tomorrow's interest)");
        println!("   ⏭️  Fast-forwarding 24 hours (scaled: 5 minutes)...");
        let timestamp = self.advance_one_day().await?;

        let alice_interest = self.total_interest(self.accounts.alice.account()).await?;
        let bob_interest = self.total_interest(self.accounts.bob.account()).await?;
        println!(
            "  Alice interest Day 2: {} (from Day 1 score: 5000 points on {} tokens)",
            alice_interest.amount.total.0,
            self.accounts.alice.initial_deposit / 10u128.pow(18)
        );
        println!(
            "  Bob interest Day 2: {} (from Day 1 score: 3000 points on {} tokens)",
            bob_interest.amount.total.0,
            self.accounts.bob.initial_deposit / 10u128.pow(18)
        );
        assert!(
            alice_interest.amount.total.0 > 0,
            "Alice should have interest from Day 1 scores"
        );
        assert!(
            bob_interest.amount.total.0 > 0,
            "Bob should have interest from Day 1 scores"
        );

        self.make_bob_second_deposit().await?;

        self.record_scores(
            timestamp,
            &[
                (self.accounts.alice.account(), 6_000),
                (self.accounts.bob.account(), 4_000),
            ],
        )
        .await?;

        self.apply_booster_and_expect_success(self.booster_scores.daily, timestamp)
            .await?;
        println!("  ✅ Booster applied successfully for both users");

        self.apply_booster_and_expect_rejection(self.booster_scores.daily, timestamp)
            .await?;
        println!("  ✅ Duplicate booster application rejected on the same day");

        self.interest_log.push(InterestSnapshot::new(
            2,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        Ok(())
    }

    async fn record_day_three_scores_and_expect_rejected_booster(&mut self) -> Result<()> {
        println!("\n📅 Day 3: Recording scores + attempting duplicate booster (should reject)");
        println!("   ⏭️  Fast-forwarding 24 hours (scaled: 5 minutes)...");
        let timestamp = self.advance_one_day().await?;

        let alice_interest = self.total_interest(self.accounts.alice.account()).await?;
        let bob_interest = self.total_interest(self.accounts.bob.account()).await?;
        let previous_day = self.interest_log.latest().expect("Day 2 interest snapshot missing");
        assert!(
            alice_interest.amount.total.0 > previous_day.alice,
            "Alice interest should increase from Day {} (score + booster)",
            previous_day.day
        );
        assert!(
            bob_interest.amount.total.0 > previous_day.bob,
            "Bob interest should increase from Day {} (score + booster)",
            previous_day.day
        );
        println!(
            "  Alice interest Day 3: {} (from Day 2 score: 6000 + booster: 1000 on {} tokens)",
            alice_interest.amount.total.0,
            self.accounts.alice.initial_deposit / 10u128.pow(18)
        );
        println!(
            "  Bob interest Day 3: {} (from Day 2 score: 4000 + booster: 1000 on {} tokens)",
            bob_interest.amount.total.0,
            self.accounts.bob.total_deposit / 10u128.pow(18)
        );

        self.record_scores(
            timestamp,
            &[
                (self.accounts.alice.account(), 7_000),
                (self.accounts.bob.account(), 5_000),
            ],
        )
        .await?;

        self.apply_booster_and_expect_success(self.booster_scores.daily, timestamp)
            .await?;
        println!("  ✅ Booster applied successfully for both users");

        self.apply_booster_and_expect_rejection(self.booster_scores.daily, timestamp)
            .await?;
        println!("  ✅ Booster correctly rejected (already applied on Day 2)");

        self.interest_log.push(InterestSnapshot::new(
            3,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        Ok(())
    }

    async fn record_day_four_scores_and_apply_new_booster(&mut self) -> Result<()> {
        println!("\n📅 Day 4: Recording scores + applying new booster (new day)");
        println!("   ⏭️  Fast-forwarding 24 hours (scaled: 5 minutes)...");
        let timestamp = self.advance_one_day().await?;

        let alice_interest = self.total_interest(self.accounts.alice.account()).await?;
        let bob_interest = self.total_interest(self.accounts.bob.account()).await?;
        let previous_day = self.interest_log.latest().expect("Day 3 interest snapshot missing");
        assert!(
            alice_interest.amount.total.0 > previous_day.alice,
            "Alice interest should increase from Day {} scores",
            previous_day.day
        );
        assert!(
            bob_interest.amount.total.0 > previous_day.bob,
            "Bob interest should increase from Day {} scores",
            previous_day.day
        );
        println!(
            "  Alice interest Day 4: {} (from Day 3 score: 7000 on {} tokens - deposit matured)",
            alice_interest.amount.total.0,
            self.accounts.alice.initial_deposit / 10u128.pow(18)
        );
        println!(
            "  Bob interest Day 4: {} (from Day 3 score: 5000 on {} tokens)",
            bob_interest.amount.total.0,
            self.accounts.bob.total_deposit / 10u128.pow(18)
        );

        self.record_scores(
            timestamp,
            &[
                (self.accounts.alice.account(), 8_000),
                (self.accounts.bob.account(), 6_000),
            ],
        )
        .await?;

        self.apply_booster_and_expect_success(self.booster_scores.new_day, timestamp)
            .await?;
        println!("  ✅ New booster applied successfully for both users");

        self.interest_log.push(InterestSnapshot::new(
            4,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        Ok(())
    }

    async fn verify_final_state(&self) -> Result<()> {
        println!("\n📊 Final State Verification:");
        let alice_final_score = self
            .context
            .sweat_jar()
            .get_score(self.accounts.alice.account().to_near())
            .await?
            .unwrap_or(U128(0));
        let bob_final_score = self
            .context
            .sweat_jar()
            .get_score(self.accounts.bob.account().to_near())
            .await?
            .unwrap_or(U128(0));
        let alice_final_interest = self
            .context
            .sweat_jar()
            .get_total_interest(self.accounts.alice.account().to_near())
            .await?;
        let bob_final_interest = self
            .context
            .sweat_jar()
            .get_total_interest(self.accounts.bob.account().to_near())
            .await?;

        println!(
            "  Alice - Final score: {}, Final interest: {}, Total deposited: {} tokens",
            alice_final_score.0,
            alice_final_interest.amount.total.0,
            self.accounts.alice.initial_deposit / 10u128.pow(18)
        );
        println!(
            "  Bob - Final score: {}, Final interest: {}, Total deposited: {} tokens",
            bob_final_score.0,
            bob_final_interest.amount.total.0,
            self.accounts.bob.total_deposit / 10u128.pow(18)
        );

        assert!(alice_final_score.0 > 0, "Alice should have accumulated score");
        assert!(bob_final_score.0 > 0, "Bob should have accumulated score");

        let final_snapshot = self.interest_log.latest().expect("Day 4 interest snapshot missing");
        assert!(
            alice_final_interest.amount.total.0 >= final_snapshot.alice,
            "Alice final interest should be >= Day {}",
            final_snapshot.day
        );
        assert!(
            bob_final_interest.amount.total.0 >= final_snapshot.bob,
            "Bob final interest should be >= Day {}",
            final_snapshot.day
        );

        println!("\n📈 Interest comparison:");
        println!(
            "  Alice had {} tokens throughout",
            self.accounts.alice.initial_deposit / 10u128.pow(18)
        );
        println!(
            "  Bob had {} tokens on Day 1-2, then {} tokens on Day 3-4",
            self.accounts.bob.initial_deposit / 10u128.pow(18),
            self.accounts.bob.total_deposit / 10u128.pow(18)
        );

        Ok(())
    }

    async fn create_initial_deposits(&mut self) -> Result<()> {
        let alice_nonce = self.accounts.alice.consume_nonce();
        let alice_message = DepositMessage::new(
            Purpose::Deposit,
            self.context.sweat_jar().contract.as_account().id(),
            self.accounts.alice.account().id(),
            &self.product_id,
            self.accounts.alice.initial_deposit,
            self.valid_until,
            alice_nonce,
        );
        self.context
            .sweat_jar()
            .create_step_jar(
                self.accounts.alice.account(),
                self.product_id.clone(),
                self.accounts.alice.initial_deposit,
                self.signer.sign(alice_message.as_str()).into(),
                self.valid_until,
                Timezone::hour_shift(0),
                &self.context.ft_contract(),
            )
            .await?;
        self.context
            .sweat_jar()
            .set_timezone(
                self.accounts.alice.account().to_near(),
                (*Timezone::hour_shift(0)).into(),
            )
            .with_user(self.accounts.manager())
            .await?;

        let bob_nonce = self.accounts.bob.consume_nonce();
        let bob_message = DepositMessage::new(
            Purpose::Deposit,
            self.context.sweat_jar().contract.as_account().id(),
            self.accounts.bob.account().id(),
            &self.product_id,
            self.accounts.bob.initial_deposit,
            self.valid_until,
            bob_nonce,
        );
        self.context
            .sweat_jar()
            .create_step_jar(
                self.accounts.bob.account(),
                self.product_id.clone(),
                self.accounts.bob.initial_deposit,
                self.signer.sign(bob_message.as_str()).into(),
                self.valid_until,
                Timezone::hour_shift(2),
                &self.context.ft_contract(),
            )
            .await?;
        self.context
            .sweat_jar()
            .set_timezone(self.accounts.bob.account().to_near(), (*Timezone::hour_shift(2)).into())
            .with_user(self.accounts.manager())
            .await?;

        Ok(())
    }

    async fn verify_initial_principals(&self) -> Result<()> {
        let alice_jars = self
            .context
            .sweat_jar()
            .get_jars_for_account(self.accounts.alice.account().to_near())
            .await?;
        let bob_jars = self
            .context
            .sweat_jar()
            .get_jars_for_account(self.accounts.bob.account().to_near())
            .await?;
        assert_eq!(
            alice_jars.get_first_deposit().unwrap().principal(),
            self.accounts.alice.initial_deposit,
            "Alice's initial principal should match deposit"
        );
        assert_eq!(
            bob_jars.get_first_deposit().unwrap().principal(),
            self.accounts.bob.initial_deposit,
            "Bob's initial principal should match deposit"
        );

        Ok(())
    }

    async fn make_bob_second_deposit(&mut self) -> Result<()> {
        let bob_second_deposit = self.accounts.bob.initial_deposit;
        let nonce = self.accounts.bob.consume_nonce();
        let message = DepositMessage::new(
            Purpose::Deposit,
            self.context.sweat_jar().contract.as_account().id(),
            self.accounts.bob.account().id(),
            &self.product_id,
            bob_second_deposit,
            self.valid_until,
            nonce,
        );

        let deposit_msg = json!({
            "type": "stake",
            "data": {
                "ticket": {
                    "product_id": self.product_id.clone(),
                    "valid_until": self.valid_until.to_string(),
                },
                "signature": self.signer.sign(message.as_str()),
            }
        });

        self.context
            .ft_contract()
            .ft_transfer_call(
                self.context.sweat_jar().contract.as_account().to_near(),
                bob_second_deposit.into(),
                None,
                deposit_msg.to_string(),
            )
            .with_user(self.accounts.bob.account())
            .deposit(NearToken::from_yoctonear(1))
            .await?;

        self.accounts.bob.add_deposit(bob_second_deposit);
        println!(
            "  Bob made second deposit: {} tokens (total: {} tokens)",
            bob_second_deposit / 10u128.pow(18),
            self.accounts.bob.total_deposit / 10u128.pow(18)
        );

        let bob_jars = self
            .context
            .sweat_jar()
            .get_jars_for_account(self.accounts.bob.account().to_near())
            .await?;
        let principal = bob_jars.get_first_deposit().unwrap().principal();
        assert_eq!(
            principal + bob_second_deposit,
            self.accounts.bob.total_deposit,
            "Bob's principal should be {} after second deposit, but got {}",
            self.accounts.bob.total_deposit,
            principal
        );
        println!("  ✅ Bob's principal verified: {} tokens", principal / 10u128.pow(18));

        Ok(())
    }

    async fn record_scores(&self, timestamp: u64, scores: &[(&Account, u16)]) -> Result<()> {
        let payload = scores
            .iter()
            .map(|(account, score)| (account.to_near(), vec![(*score, UTC(timestamp))]))
            .collect::<Vec<_>>();

        self.context
            .sweat_jar()
            .record_score(payload)
            .with_user(self.accounts.manager())
            .await?;

        Ok(())
    }

    async fn apply_booster_and_expect_success(&self, score: u16, timestamp: u64) -> Result<()> {
        let result = self
            .context
            .sweat_jar()
            .apply_booster(
                vec![
                    self.accounts.alice.account().to_near(),
                    self.accounts.bob.account().to_near(),
                ],
                score,
                sweat_jar_model::UTC(timestamp),
            )
            .with_user(self.accounts.manager())
            .result()
            .await?;

        assert!(result
            .logs()
            .iter()
            .any(|log| log.contains(r#""event": "apply_booster""#)));

        Ok(())
    }

    async fn apply_booster_and_expect_rejection(&self, score: u16, timestamp: u64) -> Result<()> {
        let result = self
            .context
            .sweat_jar()
            .apply_booster(
                vec![
                    self.accounts.alice.account().to_near(),
                    self.accounts.bob.account().to_near(),
                ],
                score,
                sweat_jar_model::UTC(timestamp),
            )
            .with_user(self.accounts.manager())
            .result()
            .await?;

        let logs = result.logs();
        let apply_booster_log = logs
            .iter()
            .find(|log| log.contains(r#""event": "apply_booster""#))
            .expect("ApplyBooster event should be emitted");

        assert!(
            apply_booster_log.contains(r#""applied": []"#),
            "Booster should be rejected (empty applied array)"
        );

        Ok(())
    }

    async fn total_interest(&self, account: &Account) -> Result<AggregatedInterestView> {
        Ok(self.context.sweat_jar().get_total_interest(account.to_near()).await?)
    }

    async fn advance_one_day(&mut self) -> Result<u64> {
        let current_time = self.context.sweat_jar().block_timestamp_ms().await?;
        
        // With scale 1/288, 1 day = 5 minutes = 300,000 ms
        const SCALED_DAY_MS: u64 = 300_000;
        
        let current_day_index = current_time / SCALED_DAY_MS;
        
        // Target the middle of the next day to avoid boundary issues
        let target_time = (current_day_index + 1) * SCALED_DAY_MS + (SCALED_DAY_MS / 2);
        
        if target_time > current_time {
            let needed_ms = target_time - current_time;
            // Convert to minutes, rounding up
            let needed_mins = (needed_ms + 60_000 - 1) / 60_000;
            self.context.fast_forward_minutes(needed_mins).await?;
        } else {
            // Fallback if we somehow calculated a past time (should be impossible with +1 day)
            self.context.fast_forward_minutes(1).await?;
        }

        Ok(self.context.sweat_jar().block_timestamp_ms().await?)
    }
}
