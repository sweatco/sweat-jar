use near_sdk::{
    json_types::{Base64VecU8, U128},
    log, near, serde_json, AccountId,
};
use sweat_jar_model::{Local, ProductId, Score, TokenAmount, U32, UTC};

use crate::{common::Timestamp, env, product::model::Product, PACKAGE_NAME, VERSION};

#[derive(Debug)]
#[near(serializers=[json])]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum EventKind {
    RegisterProduct(Product),
    Deposit((ProductId, U128)),
    Claim(ClaimData),
    Withdraw(WithdrawData),
    WithdrawAll(Vec<WithdrawData>),
    Restake(RestakeData),
    RestakeAll(RestakeAllData),
    ApplyPenalty(PenaltyData),
    BatchApplyPenalty(BatchPenaltyData),
    EnableProduct(EnableProductData),
    ChangeProductPublicKey(ChangeProductPublicKeyData),
    RecordScore(Vec<ScoreData>),
    OldScoreWarning((Score, Local)),
    JarsMerge(AccountId),
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
pub type ClaimEventItem = (ProductId, U128);

#[derive(Default, Debug)]
#[near(serializers=[json])]
pub struct ClaimData {
    pub timestamp: Timestamp,
    pub items: Vec<ClaimEventItem>,
}

/// (id, fee, amount)
pub type WithdrawData = (ProductId, U128, U128);

// TODO: doc change
#[derive(Debug)]
#[near(serializers=[json])]
pub struct RestakeData {
    pub product_id: ProductId,
    pub restaked: U128,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub struct RestakeAllData {
    pub timestamp: Timestamp,
    pub from: Vec<ProductId>,
    pub into: ProductId,
    pub restaked: U128,
    pub withdrawn: U128,
}

impl RestakeAllData {
    pub fn new(
        timestamp: Timestamp,
        from: Vec<ProductId>,
        into: ProductId,
        restaked: TokenAmount,
        withdrawn: TokenAmount,
    ) -> Self {
        RestakeAllData {
            timestamp,
            from,
            into,
            restaked: restaked.into(),
            withdrawn: withdrawn.into(),
        }
    }
}

impl RestakeData {
    pub fn new(product_id: ProductId, restaked: TokenAmount) -> Self {
        RestakeData {
            product_id,
            restaked: restaked.into(),
        }
    }
}

#[derive(Debug)]
#[near(serializers=[json])]
// TODO: doc change
pub struct PenaltyData {
    pub account_id: AccountId,
    pub is_applied: bool,
    pub timestamp: Timestamp,
}

#[derive(Debug)]
#[near(serializers=[json])]
// TODO: doc change
pub struct BatchPenaltyData {
    pub account_ids: Vec<AccountId>,
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
    use near_sdk::json_types::U128;
    use sweat_jar_model::Local;

    use crate::{
        common::tests::{Context, WhitespaceTrimmer},
        event::{ClaimData, EventKind, SweatJarEvent},
        test_utils::admin,
    };

    #[test]
    fn test_contract_version() {
        let admin = admin();
        let context = Context::new(admin);
        assert_eq!(context.contract().contract_version(), "sweat_jar-3.3.10");
    }

    #[test]
    fn event_to_string() {
        let event = SweatJarEvent::from(EventKind::Claim(ClaimData {
            timestamp: 1234567,
            items: vec![
                ("product_0".to_string(), U128(50)),
                ("product_1".to_string(), U128(200)),
            ],
        }))
        .to_json_event_string();
        let json = r#"EVENT_JSON:{
          "standard": "sweat_jar",
          "version": "3.3.10",
          "event": "claim",
          "data": {
            "timestamp": 1234567,
            "items": [ [ "product_0", "50" ], [ "product_1", "200" ] ]
          }
        }"#;

        assert_eq!(json.trim_whitespaces(), event.trim_whitespaces());

        let event = SweatJarEvent::from(EventKind::OldScoreWarning((111, Local(5)))).to_json_event_string();
        let json = r#"EVENT_JSON:{
          "standard": "sweat_jar",
          "version": "3.3.10",
          "event": "old_score_warning",
          "data": [ 111, 5 ]
        }"#;

        assert_eq!(json.trim_whitespaces(), event.trim_whitespaces());
    }
}
