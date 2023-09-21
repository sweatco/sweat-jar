use near_sdk::{
    json_types::{Base64VecU8, U128},
    log,
    serde::{Deserialize, Serialize},
    serde_json, AccountId,
};

use crate::{
    env,
    jar::model::{Jar, JarID},
    product::model::{Product, ProductId},
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
    CreateJar(Jar),
    Claim(Vec<ClaimEventItem>),
    Withdraw(WithdrawData),
    Migration(Vec<MigrationEventItem>),
    Restake(RestakeData),
    ApplyPenalty(PenaltyData),
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
    pub index: JarID,
    pub interest_to_claim: U128,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct WithdrawData {
    pub id: JarID,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct MigrationEventItem {
    pub original_id: String,
    pub index: JarID,
    pub account_id: AccountId,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct RestakeData {
    pub old_index: JarID,
    pub new_index: JarID,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PenaltyData {
    pub index: JarID,
    pub is_applied: bool,
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
    pub index: JarID,
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

pub(crate) fn emit(event: EventKind) {
    SweatJarEvent::from(event).emit();
}

impl SweatJarEvent {
    pub(crate) fn emit(&self) {
        log!(self.to_json_event_string());
    }

    fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|err| env::panic_str(&format!("Failed to serialize SweatJarEvent: {err}")))
    }

    fn to_json_event_string(&self) -> String {
        format!("EVENT_JSON:{}", self.to_json_string())
    }
}
