use near_sdk::{near, AccountId};
use sweat_jar_model::{jar::JarId, TokenAmount};

use crate::common::Timestamp;

#[near(serializers=[borsh, json])]
pub struct ClaimData {
    pub account_id: AccountId,
    pub now: Timestamp,
    pub jars: Vec<ClaimJar>,
}

#[near(serializers=[borsh, json])]
pub struct ClaimJar {
    pub jar_id: JarId,
    pub available_yield: TokenAmount,
    pub claimed_amount: TokenAmount,
}
