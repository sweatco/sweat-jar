use anyhow::Result;
use near_workspaces::Account;
use sweat_jar_model::{
    data::{
        deposit::{DepositMessage, Purpose},
        jar::AggregatedInterestView,
        product::{Cap, Product, Terms, TieredScoreBasedProductTerms},
    },
    signer::test_utils::MessageSigner,
    MS_IN_DAY,
};
use tracing::info;

mod common;
use common::{
    jar,
    prepare::{prepare_contract, Context as TestContext},
};

#[tokio::test]
#[tracing::instrument]
async fn test_multi_day_score_and_booster_recording() -> Result<()> {
    common::prepare::init_tracing();
    info!("multi-day score and booster recording test (1 day = 5 minutes, 288x speed)");

    let mut scenario = MultiDayBoosterScenario::new().await?;

    scenario.record_day_one_scores().await?;
    scenario.record_day_two_scores_and_apply_booster().await?;
    scenario.record_day_three_scores_and_apply_booster().await?;
    scenario.record_day_four_scores_and_apply_new_booster().await?;
    scenario.verify_final_state().await?;

    info!("multi-day score and booster recording test completed successfully");
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
    next_nonce: u32,
}

impl ParticipantState {
    fn new(account: Account, initial_deposit: u128) -> Self {
        Self {
            account,
            initial_deposit,
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

        let context = prepare_contract([]).await?;

        info!("setting time scale: 1 day = 5 minutes");
        jar::set_time_scale(&context.jar, &context.manager, 1.0 / 288.0).await?;

        jar::register_product(&context.jar, &context.manager, product.clone()).await?;

        let accounts = Accounts {
            manager: context.manager.clone(),
            alice: ParticipantState::new(context.alice.clone(), base_deposit),
            bob: ParticipantState::new(context.bob.clone(), base_deposit),
        };

        // Signed tickets must satisfy the on-chain upper bound on `valid_until`
        // (at most 7 real days from now), so anchor it to the chain's clock.
        let valid_until = jar::block_timestamp_ms(&context.jar).await? + MS_IN_DAY;

        let mut scenario = Self {
            context,
            signer,
            product_id,
            accounts,
            booster_scores: BoosterScores::new(1_000, 1_500),
            valid_until,
            interest_log: InterestLog::new(),
        };

        scenario.create_initial_deposits().await?;

        Ok(scenario)
    }

    async fn record_day_one_scores(&mut self) -> Result<()> {
        info!("day 1: recording scores (will affect tomorrow's interest)");
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

        self.interest_log.push(InterestSnapshot::new(
            1,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        self.verify_initial_principals().await?;

        Ok(())
    }

    async fn record_day_two_scores_and_apply_booster(&mut self) -> Result<()> {
        info!("day 2: recording scores + applying booster");
        let timestamp = self.advance(2).await?;

        let alice_interest = self.total_interest(self.accounts.alice.account()).await?;
        let bob_interest = self.total_interest(self.accounts.bob.account()).await?;
        let previous_snapshot = self.interest_log.latest().expect("Day 1 snapshot missing");
        assert!(
            alice_interest.amount.total.0 >= previous_snapshot.alice,
            "Alice interest should be >= Day 1"
        );
        assert!(
            bob_interest.amount.total.0 >= previous_snapshot.bob,
            "Bob interest should be >= Day 1"
        );

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
        self.apply_booster_and_expect_rejection(self.booster_scores.daily, timestamp)
            .await?;

        self.interest_log.push(InterestSnapshot::new(
            2,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        Ok(())
    }

    async fn record_day_three_scores_and_apply_booster(&mut self) -> Result<()> {
        info!("day 3: recording scores + applying booster");
        let timestamp = self.advance_one_day().await?;

        let alice_interest = self.total_interest(self.accounts.alice.account()).await?;
        let bob_interest = self.total_interest(self.accounts.bob.account()).await?;
        let previous_day = self.interest_log.latest().expect("Day 2 interest snapshot missing");
        assert!(
            alice_interest.amount.total.0 >= previous_day.alice,
            "Alice interest should be >= Day {}",
            previous_day.day
        );
        assert!(
            bob_interest.amount.total.0 >= previous_day.bob,
            "Bob interest should be >= Day {}",
            previous_day.day
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
        self.apply_booster_and_expect_rejection(self.booster_scores.daily, timestamp)
            .await?;

        self.interest_log.push(InterestSnapshot::new(
            3,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        Ok(())
    }

    async fn record_day_four_scores_and_apply_new_booster(&mut self) -> Result<()> {
        info!("day 4: recording scores + applying new booster");
        let timestamp = self.advance_one_day().await?;

        let alice_interest = self.total_interest(self.accounts.alice.account()).await?;
        let bob_interest = self.total_interest(self.accounts.bob.account()).await?;
        let previous_day = self.interest_log.latest().expect("Day 3 interest snapshot missing");
        assert!(
            alice_interest.amount.total.0 >= previous_day.alice,
            "Alice interest should be >= Day {} scores",
            previous_day.day
        );
        assert!(
            bob_interest.amount.total.0 >= previous_day.bob,
            "Bob interest should be >= Day {} scores",
            previous_day.day
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

        self.interest_log.push(InterestSnapshot::new(
            4,
            alice_interest.amount.total.0,
            bob_interest.amount.total.0,
        ));

        Ok(())
    }

    async fn verify_final_state(&self) -> Result<()> {
        let alice_final_score = jar::get_score(&self.context.jar, self.accounts.alice.account().id())
            .await?
            .unwrap_or(0);
        let bob_final_score = jar::get_score(&self.context.jar, self.accounts.bob.account().id())
            .await?
            .unwrap_or(0);
        let alice_final_interest = jar::get_total_interest(&self.context.jar, self.accounts.alice.account().id()).await?;
        let bob_final_interest = jar::get_total_interest(&self.context.jar, self.accounts.bob.account().id()).await?;

        assert!(alice_final_score > 0, "Alice should have accumulated score");
        assert!(bob_final_score > 0, "Bob should have accumulated score");

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

        Ok(())
    }

    async fn create_initial_deposits(&mut self) -> Result<()> {
        let alice_nonce = self.accounts.alice.consume_nonce();
        let alice_message = DepositMessage::new(
            Purpose::Deposit,
            self.context.jar.id(),
            self.accounts.alice.account().id(),
            &self.product_id,
            self.accounts.alice.initial_deposit,
            self.valid_until,
            alice_nonce,
        );
        jar::create_step_jar(
            &self.context.jar,
            &self.context.ft,
            self.accounts.alice.account(),
            &self.product_id,
            self.accounts.alice.initial_deposit,
            self.signer.sign(alice_message.as_str()).into(),
            self.valid_until,
            0,
        )
        .await?;
        jar::set_timezone(&self.context.jar, self.accounts.manager(), self.accounts.alice.account().id(), 0).await?;

        let bob_nonce = self.accounts.bob.consume_nonce();
        let bob_message = DepositMessage::new(
            Purpose::Deposit,
            self.context.jar.id(),
            self.accounts.bob.account().id(),
            &self.product_id,
            self.accounts.bob.initial_deposit,
            self.valid_until,
            bob_nonce,
        );
        jar::create_step_jar(
            &self.context.jar,
            &self.context.ft,
            self.accounts.bob.account(),
            &self.product_id,
            self.accounts.bob.initial_deposit,
            self.signer.sign(bob_message.as_str()).into(),
            self.valid_until,
            2,
        )
        .await?;
        jar::set_timezone(&self.context.jar, self.accounts.manager(), self.accounts.bob.account().id(), 2).await?;

        Ok(())
    }

    async fn verify_initial_principals(&self) -> Result<()> {
        let alice_jars = jar::get_jars_for_account(&self.context.jar, self.accounts.alice.account().id()).await?;
        let bob_jars = jar::get_jars_for_account(&self.context.jar, self.accounts.bob.account().id()).await?;
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

    async fn record_scores(&self, timestamp: u64, scores: &[(&Account, u16)]) -> Result<()> {
        let payload = scores
            .iter()
            .map(|(account, score)| (account.id().clone(), vec![(*score, timestamp)]))
            .collect::<Vec<_>>();

        jar::record_score(&self.context.jar, self.accounts.manager(), payload).await?;

        Ok(())
    }

    async fn apply_booster_and_expect_success(&self, score: u16, timestamp: u64) -> Result<()> {
        let result = jar::apply_booster(
            &self.context.jar,
            self.accounts.manager(),
            vec![
                self.accounts.alice.account().id().clone(),
                self.accounts.bob.account().id().clone(),
            ],
            score,
            timestamp,
        )
        .await?;

        assert!(result.logs().iter().any(|log| log.contains(r#""event": "apply_booster""#)));

        Ok(())
    }

    async fn apply_booster_and_expect_rejection(&self, score: u16, timestamp: u64) -> Result<()> {
        let result = jar::apply_booster(
            &self.context.jar,
            self.accounts.manager(),
            vec![
                self.accounts.alice.account().id().clone(),
                self.accounts.bob.account().id().clone(),
            ],
            score,
            timestamp,
        )
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
        jar::get_total_interest(&self.context.jar, account.id()).await
    }

    async fn advance(&mut self, days: u16) -> Result<u64> {
        let current_time = jar::block_timestamp_ms(&self.context.jar).await?;

        const SCALED_DAY_MS: u64 = 300_000;
        let current_day_index = current_time / SCALED_DAY_MS;
        let target_time = (current_day_index + u64::from(days)) * SCALED_DAY_MS + (SCALED_DAY_MS / 2);

        if target_time > current_time {
            let needed_ms = target_time - current_time;
            let needed_mins = (needed_ms + 60_000 - 1) / 60_000;
            self.context.fast_forward_minutes(needed_mins).await?;
        } else {
            self.context.fast_forward_minutes(1).await?;
        }

        jar::block_timestamp_ms(&self.context.jar).await
    }

    async fn advance_one_day(&mut self) -> Result<u64> {
        self.advance(1).await
    }
}
