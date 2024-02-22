use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    store::LookupMap,
    AccountId,
};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::{
    borsh,
    common::Timestamp,
    jar::model::{Jar, JarCache, JarV1},
    AccountJars,
};

#[derive(
BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
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
        JarV1 {
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

#[derive(Default, Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct AccountJarsLegacy {
    pub last_id: JarId,
    pub jars: Vec<JarLegacy>,
}

impl From<AccountJarsLegacy> for AccountJars {
    #[mutants::skip]
    fn from(value: AccountJarsLegacy) -> Self {
        AccountJars {
            last_id: value.last_id,
            jars: value.jars.into_iter().map(Into::into).collect(),
        }
    }
}

pub type AccountJarsMapLegacy = LookupMap<AccountId, AccountJarsLegacy>;
