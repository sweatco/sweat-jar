use near_sdk::{
    json_types::{Base64VecU8, U128},
    log,
    serde::{Deserialize, Serialize},
    serde_json, AccountId,
};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::{
    common::Timestamp,
    env,
    jar::model::{JarCache, JarV1},
    product::model::Product,
    PACKAGE_NAME, VERSION,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(
    crate = "near_sdk::serde",
    tag = "event",
    content = "data",
    rename_all = "snake_case"
)]
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
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
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

impl From<JarV1> for EventJar {
    fn from(value: JarV1) -> Self {
        Self {
            id: value.id,
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: value.created_at,
            principal: value.principal,
            cache: value.cache,
            claimed_balance: value.claimed_balance,
            is_pending_withdraw: value.is_pending_withdraw,
            is_penalty_applied: value.is_penalty_applied,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
struct SweatJarEvent {
    standard: &'static str,
    version: &'static str,
    #[serde(flatten)]
    event_kind: EventKind,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ClaimEventItem {
    pub id: JarId,
    pub interest_to_claim: U128,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct WithdrawData {
    pub id: JarId,
    pub fee: U128,
    pub amount: U128,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct MigrationEventItem {
    pub original_id: String,
    pub id: JarId,
    pub account_id: AccountId,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct RestakeData {
    pub old_id: JarId,
    pub new_id: JarId,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PenaltyData {
    pub id: JarId,
    pub is_applied: bool,
    pub timestamp: Timestamp,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct BatchPenaltyData {
    pub jars: Vec<JarId>,
    pub is_applied: bool,
    pub timestamp: Timestamp,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct EnableProductData {
    pub id: ProductId,
    pub is_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ChangeProductPublicKeyData {
    pub product_id: ProductId,
    pub pk: Base64VecU8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct TopUpData {
    pub id: JarId,
    pub amount: U128,
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
    log!(SweatJarEvent::from(event).to_json_event_string());
}

#[mutants::skip]
#[cfg(test)]
pub(crate) fn emit(event: EventKind) {
    if crate::common::test_data::should_log_events() {
        log!(SweatJarEvent::from(event).to_json_event_string());
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

    use crate::{
        common::tests::Context,
        event::{EventKind, SweatJarEvent, TopUpData},
        jar::model::JarV1,
        test_utils::admin,
    };

    #[test]
    fn test_contract_version() {
        let admin = admin();
        let context = Context::new(admin);
        assert_eq!(context.contract().contract_version(), "sweat_jar-2.2.0");
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
  "version": "2.2.0",
  "event": "top_up",
  "data": {
    "id": 10,
    "amount": "50"
  }
}"#
        );

        assert_eq!(
            SweatJarEvent::from(EventKind::CreateJar(
                JarV1 {
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
                }
                .into()
            ))
            .to_json_event_string(),
            r#"EVENT_JSON:{
  "standard": "sweat_jar",
  "version": "2.2.0",
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
    }
}
