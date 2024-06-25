use near_sdk::json_types::{U128, U64};
use sweat_jar_model::{jar::JarView, U32};

use crate::jar::model::Jar;

impl From<Jar> for JarView {
    fn from(value: Jar) -> Self {
        Self {
            id: U32(value.id),
            account_id: value.account_id.clone(),
            product_id: value.product_id.clone(),
            created_at: U64(value.created_at),
            principal: U128(value.principal),
            claimed_balance: U128(value.claimed_balance),
            is_penalty_applied: value.is_penalty_applied,
            is_pending_withdraw: value.is_pending_withdraw,
        }
    }
}

impl From<&Jar> for JarView {
    fn from(value: &Jar) -> Self {
        Self {
            id: U32(value.id),
            account_id: value.account_id.clone(),
            product_id: value.product_id.clone(),
            created_at: U64(value.created_at),
            principal: U128(value.principal),
            claimed_balance: U128(value.claimed_balance),
            is_penalty_applied: value.is_penalty_applied,
            is_pending_withdraw: value.is_pending_withdraw,
        }
    }
}
