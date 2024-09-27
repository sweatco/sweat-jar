use near_sdk::{
    json_types::{Base64VecU8, U128},
    log, near, serde_json, AccountId,
};
use sweat_jar_model::{jar::JarId, Local, ProductId, Score, TokenAmount, U32, UTC};

use crate::{
    common::Timestamp,
    env,
    jar::model::{Jar, JarCache},
    product::model::Product,
    PACKAGE_NAME, VERSION,
};

#[derive(Debug)]
#[near(serializers=[json])]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum EventKind {
    RegisterProduct(Product),
    CreateJar(EventJar),
    Claim(Vec<ClaimEventItem>),
    Withdraw(WithdrawData),
    WithdrawAll(Vec<WithdrawData>),
    Migration(Vec<MigrationEventItem>),
    Restake(RestakeData),
    RestakeAll(Vec<RestakeData>),
    ApplyPenalty(PenaltyData),
    BatchApplyPenalty(BatchPenaltyData),
    EnableProduct(EnableProductData),
    ChangeProductPublicKey(ChangeProductPublicKeyData),
    TopUp(TopUpData),
    RecordScore(Vec<ScoreData>),
    OldScoreWarning((Score, Local)),
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct EventJar {
    id: JarId,
    account_id: AccountId,
    product_id: ProductId,
    created_at: Timestamp,
    principal: TokenAmount,
    cache: Option<JarCache>,
    claimed_balance: TokenAmount,
    is_pending_withdraw: bool,
    is_penalty_applied: bool,
}

impl From<Jar> for EventJar {
    fn from(jar: Jar) -> Self {
        Self {
            id: jar.id,
            account_id: jar.account_id.clone(),
            product_id: jar.product_id.clone(),
            created_at: jar.created_at,
            principal: jar.principal,
            cache: jar.cache,
            claimed_balance: jar.claimed_balance,
            is_pending_withdraw: jar.is_pending_withdraw,
            is_penalty_applied: jar.is_penalty_applied,
        }
    }
}

#[derive(Debug)]
#[near(serializers=[json])]
struct SweatJarEvent {
    standard: &'static str,
    version: &'static str,
    #[serde(flatten)]
    event_kind: EventKind,
}

/// `JarId` and interest to claim
pub type ClaimEventItem = (JarId, U128);

#[derive(Debug)]
#[near(serializers=[json])]
pub struct WithdrawData {
    pub id: JarId,
    pub fee: U128,
    pub amount: U128,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct MigrationEventItem {
    pub original_id: String,
    pub id: JarId,
    pub account_id: AccountId,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct RestakeData {
    pub jar_id: JarId,
    pub amount: TokenAmount,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct PenaltyData {
    pub id: JarId,
    pub is_applied: bool,
    pub timestamp: Timestamp,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct BatchPenaltyData {
    pub jars: Vec<JarId>,
    pub is_applied: bool,
    pub timestamp: Timestamp,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct EnableProductData {
    pub id: ProductId,
    pub is_enabled: bool,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct ChangeProductPublicKeyData {
    pub product_id: ProductId,
    pub pk: Base64VecU8,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct TopUpData {
    pub id: JarId,
    pub amount: U128,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct ScoreData {
    pub account_id: AccountId,
    pub score: Vec<(U32, UTC)>,
}

impl From<EventKind> for SweatJarEvent {
    fn from(event_kind: EventKind) -> Self {
        Self {
            standard: PACKAGE_NAME,
            version: VERSION,
            event_kind,
        }
    }
}

#[mutants::skip]
#[cfg(not(test))]
pub(crate) fn emit(event: EventKind) {
    log!("{}", SweatJarEvent::from(event).to_json_event_string());
}

#[mutants::skip]
#[cfg(test)]
pub(crate) fn emit(event: EventKind) {
    if crate::common::test_data::should_log_events() {
        log!("{}", SweatJarEvent::from(event).to_json_event_string());
    }
}

impl SweatJarEvent {
    fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|err| env::panic_str(&format!("Failed to serialize SweatJarEvent: {err}")))
    }

    fn to_json_event_string(&self) -> String {
        format!("EVENT_JSON:{}", self.to_json_string())
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use near_sdk::{json_types::U128, AccountId};
    use sweat_jar_model::Local;

    use crate::{
        common::tests::Context,
        event::{EventKind, ScoreData, SweatJarEvent, TopUpData},
        jar::model::{Jar, JarLastVersion},
        test_utils::admin,
    };

    #[test]
    fn test_contract_version() {
        let admin = admin();
        let context = Context::new(admin);
        assert_eq!(context.contract().contract_version(), "sweat_jar-3.3.0");
    }

    #[test]
    fn event_to_string() {
        assert_eq!(
            SweatJarEvent::from(EventKind::TopUp(TopUpData {
                id: 10,
                amount: U128(50),
            }))
            .to_json_event_string(),
            r#"EVENT_JSON:{
  "standard": "sweat_jar",
  "version": "3.3.0",
  "event": "top_up",
  "data": {
    "id": 10,
    "amount": "50"
  }
}"#
        );

        assert_eq!(
            SweatJarEvent::from(EventKind::CreateJar(
                Jar::V1(JarLastVersion {
                    id: 555,
                    account_id: "bob.near".to_string().try_into().unwrap(),
                    product_id: "some_product".to_string(),
                    created_at: 1234324235,
                    principal: 78685678567,
                    cache: None,
                    claimed_balance: 4324,
                    is_pending_withdraw: false,
                    is_penalty_applied: false,
                    claim_remainder: 55555,
                })
                .into()
            ))
            .to_json_event_string(),
            r#"EVENT_JSON:{
  "standard": "sweat_jar",
  "version": "3.3.0",
  "event": "create_jar",
  "data": {
    "id": 555,
    "account_id": "bob.near",
    "product_id": "some_product",
    "created_at": 1234324235,
    "principal": 78685678567,
    "cache": null,
    "claimed_balance": 4324,
    "is_pending_withdraw": false,
    "is_penalty_applied": false
  }
}"#
        );

        assert_eq!(
            SweatJarEvent::from(EventKind::Claim(vec![(1, 1.into()), (2, 2.into())])).to_json_event_string(),
            r#"EVENT_JSON:{
  "standard": "sweat_jar",
  "version": "3.3.0",
  "event": "claim",
  "data": [
    [
      1,
      "1"
    ],
    [
      2,
      "2"
    ]
  ]
}"#
        );

        assert_eq!(
            SweatJarEvent::from(EventKind::RecordScore(vec![
                ScoreData {
                    account_id: AccountId::from_str("alice.near").unwrap(),
                    score: vec![(10.into(), 10.into())],
                },
                ScoreData {
                    account_id: AccountId::from_str("bob.near").unwrap(),
                    score: vec![(20.into(), 20.into())],
                }
            ]))
            .to_json_event_string(),
            r#"EVENT_JSON:{
  "standard": "sweat_jar",
  "version": "3.3.0",
  "event": "record_score",
  "data": [
    {
      "account_id": "alice.near",
      "score": [
        [
          "10",
          10
        ]
      ]
    },
    {
      "account_id": "bob.near",
      "score": [
        [
          "20",
          20
        ]
      ]
    }
  ]
}"#
        );

        assert_eq!(
            SweatJarEvent::from(EventKind::OldScoreWarning((111, Local(5)))).to_json_event_string(),
            r#"EVENT_JSON:{
  "standard": "sweat_jar",
  "version": "3.3.0",
  "event": "old_score_warning",
  "data": [
    111,
    5
  ]
}"#
        );
    }
}
