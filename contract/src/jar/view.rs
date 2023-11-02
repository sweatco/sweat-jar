use model::{jar::JarView, U32};
use near_sdk::json_types::{U128, U64};

use crate::jar::model::Jar;

impl From<Jar> for JarView {
    fn from(value: Jar) -> Self {
        Self {
            id: U32(value.id),
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: U64(value.created_at),
            principal: U128(value.principal),
            claimed_balance: U128(value.claimed_balance),
            is_penalty_applied: value.is_penalty_applied,
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
        }
    }
}
