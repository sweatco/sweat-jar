use jar_model::{jar::JarId, ProductId};
use near_sdk::{
    json_types::{Base64VecU8, U128},
    log,
    serde::{Deserialize, Serialize},
    serde_json, AccountId,
};

use crate::{common::Timestamp, env, jar::model::Jar, product::model::Product, PACKAGE_NAME, VERSION};

#[derive(Serialize, Deserialize, Debug)]
#[serde(
    crate = "near_sdk::serde",
    tag = "event",
    content = "data",
    rename_all = "snake_case"
)]
pub enum EventKind {
    RegisterProduct(Product),
    CreateJar(Jar),
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
pub(crate) fn emit(event: EventKind) {
    log!(SweatJarEvent::from(event).to_json_event_string());
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

    use crate::event::{EventKind, SweatJarEvent, TopUpData};

    #[test]
    fn event_to_string() {
        assert_eq!(
            SweatJarEvent::from(EventKind::TopUp(TopUpData {
                id: 10,
                amount: U128(50)
            }))
            .to_json_event_string(),
            r#"EVENT_JSON:{
  "standard": "sweat_jar",
  "version": "1.0.0",
  "event": "top_up",
  "data": {
    "id": 10,
    "amount": "50"
  }
}"#
        )
    }
}
