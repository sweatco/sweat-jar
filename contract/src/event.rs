use near_sdk::{Balance, serde::{Serialize, Deserialize}, serde_json};

use crate::jar::JarIndex;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub(crate) enum Event {
    Claim(ClaimEvent),
    Withdraw(WithdrawEvent),
}

impl Event {
    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct ClaimEvent {
    index: JarIndex,
    amount: Balance,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct WithdrawEvent {
    index: JarIndex,
    amount: Balance,
}