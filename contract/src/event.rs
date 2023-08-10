use near_sdk::{AccountId, log, serde_json};
use near_sdk::serde::Serialize;

use crate::{PACKAGE_NAME, VERSION};
use crate::common::TokenAmount;
use crate::jar::model::{Jar, JarIndex};
use crate::product::model::Product;

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventKind {
    RegisterProduct(Product),
    CreateJar(Jar),
    Claim(Vec<ClaimEventItem>),
    Withdraw(WithdrawData),
    Migration(Vec<MigrationEventItem>),
    Restaked(RestakeData),
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(rename_all = "snake_case")]
struct SweatJarEvent {
    standard: String,
    version: String,
    #[serde(flatten)]
    event_kind: EventKind,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct ClaimEventItem {
    pub index: JarIndex,
    pub interest_to_claim: TokenAmount,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct WithdrawData {
    pub index: JarIndex,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct MigrationEventItem {
    pub original_id: String,
    pub index: JarIndex,
    pub account_id: AccountId,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct RestakeData {
    pub old_index: JarIndex,
    pub new_index: JarIndex,
}

impl From<EventKind> for SweatJarEvent {
    fn from(event_kind: EventKind) -> Self {
        Self {
            standard: PACKAGE_NAME.into(),
            version: VERSION.into(),
            event_kind,
        }
    }
}

pub(crate) fn emit(event: EventKind) {
    SweatJarEvent::from(event).emit();
}

impl SweatJarEvent {
    pub(crate) fn emit(&self) {
        log!(self.to_json_event_string())
    }

    fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn to_json_event_string(&self) -> String {
        format!("EVENT_JSON:{}", self.to_json_string())
    }
}
