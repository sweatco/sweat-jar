use near_sdk::{near, AccountId};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::{
    common::Timestamp,
    jar::model::{Jar, JarCache, JarLastVersion},
};

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub struct JarLegacy {
    pub id: JarId,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: Timestamp,
    pub principal: TokenAmount,
    pub cache: Option<JarCache>,
    pub claimed_balance: TokenAmount,
    pub is_pending_withdraw: bool,
    pub is_penalty_applied: bool,
}

impl From<JarLegacy> for Jar {
    #[mutants::skip]
    fn from(value: JarLegacy) -> Self {
        JarLastVersion {
            id: value.id,
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: value.created_at,
            principal: value.principal,
            cache: value.cache,
            claimed_balance: value.claimed_balance,
            is_pending_withdraw: value.is_pending_withdraw,
            is_penalty_applied: value.is_penalty_applied,
            claim_remainder: 0,
        }
        .into()
    }
}

#[near]
#[derive(Default, Debug, Clone)]
pub struct AccountLegacyV1 {
    pub last_id: JarId,
    pub jars: Vec<JarLegacy>,
}

#[near]
#[derive(Default, Clone)]
pub struct AccountLegacyV2 {
    pub last_id: JarId,
    pub jars: Vec<Jar>,
}

impl From<AccountLegacyV1> for AccountLegacyV2 {
    #[mutants::skip]
    fn from(value: AccountLegacyV1) -> Self {
        AccountLegacyV2 {
            last_id: value.last_id,
            jars: value.jars.into_iter().map(Into::into).collect(),
        }
    }
}
