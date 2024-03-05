use near_sdk::{
    json_types::{Base64VecU8, U128},
    log,
    serde::{Deserialize, Serialize},
    serde_json,
    serde_json::{to_value, Value},
    AccountId,
};
use sweat_jar_model::{jar::JarId, ProductId};

use crate::{common::Timestamp, env, jar::model::JarV1, product::model::Product, PACKAGE_NAME, VERSION};

#[derive(Serialize, Deserialize, Debug)]
#[serde(
    crate = "near_sdk::serde",
    tag = "event",
    content = "data",
    rename_all = "snake_case"
)]
pub enum EventKind {
    RegisterProduct(Product),
    CreateJar(JarV1),
    Claim(Vec<ClaimEventItem>),
    Withdraw(WithdrawData),
    Migration(Vec<MigrationEventItem>),
    Restake(RestakeData),
    ApplyPenalty(PenaltyData),
    BatchApplyPenalty(BatchPenaltyData),
    EnableProduct(EnableProductData),
    ChangeProductPublicKey(ChangeProductPublicKeyData),
    TopUp(TopUpData),
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
    pub fee_amount: U128,
    pub withdrawn_amount: U128,
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
    fn to_json_string(value: &Value) -> String {
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|err| env::panic_str(&format!("Failed to serialize SweatJarEvent: {err}")))
    }

    fn to_json_event_string(&self) -> String {
        let value = self.skip_fields_if_needed();
        format!("EVENT_JSON:{}", Self::to_json_string(&value))
    }

    // Events has a defined format and don't need all fields from the model
    // #[serde(skip_serializing)] can't be used in our case beceuase Serde is also used
    // for transferring data in cross contract calls
    fn skip_fields_if_needed(&self) -> Value {
        let mut value =
            to_value(self).unwrap_or_else(|err| env::panic_str(&format!("Failed to serialize SweatJarEvent: {err}")));

        let EventKind::CreateJar(_) = self.event_kind else {
            return value;
        };

        value["data"]
            .as_object_mut()
            .unwrap_or_else(|| env::panic_str(&"Failed to skip claim_remainder field"))
            .remove("claim_remainder");

        value
    }
}

#[cfg(test)]
mod test {
    use near_sdk::{json_types::U128, AccountId};

    use crate::{
        event::{EventKind, SweatJarEvent, TopUpData},
        jar::model::JarV1,
    };

    #[test]
    /// Don't forget to notify backend team if these events format is changed
    fn event_to_string() {
        assert_eq!(
            SweatJarEvent::from(EventKind::TopUp(TopUpData {
                id: 10,
                amount: U128(50),
            }))
            .to_json_event_string(),
            r#"EVENT_JSON:{
  "data": {
    "amount": "50",
    "id": 10
  },
  "event": "top_up",
  "standard": "sweat_jar",
  "version": "1.0.0"
}"#
        );

        assert_eq!(
            SweatJarEvent::from(EventKind::CreateJar(JarV1 {
                id: 555,
                account_id: AccountId::new_unchecked("bob.near".to_string()),
                product_id: "some_product".to_string(),
                created_at: 1234324235,
                principal: 78685678567,
                cache: None,
                claimed_balance: 4324,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 55555,
            }))
            .to_json_event_string(),
            r#"EVENT_JSON:{
  "data": {
    "account_id": "bob.near",
    "cache": null,
    "claimed_balance": 4324,
    "created_at": 1234324235,
    "id": 555,
    "is_penalty_applied": false,
    "is_pending_withdraw": false,
    "principal": 78685678567,
    "product_id": "some_product"
  },
  "event": "create_jar",
  "standard": "sweat_jar",
  "version": "1.0.0"
}"#
        );
    }
}
