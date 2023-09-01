use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

use near_sdk::{
    json_types::{U128, U64},
    serde::Serialize,
    AccountId,
};

use crate::{common::U32, product::model::ProductId, *};

pub type JarIndexView = U32;

#[derive(Serialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct JarView {
    pub index: JarIndexView,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
    pub claimed_balance: U128,
    pub is_penalty_applied: bool,
}

impl From<Jar> for JarView {
    fn from(value: Jar) -> Self {
        Self {
            index: U32(value.index),
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: U64(value.created_at),
            principal: U128(value.principal),
            claimed_balance: U128(value.claimed_balance),
            is_penalty_applied: value.is_penalty_applied,
        }
    }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<JarIndexView, U128>,
    pub total: U128,
}

#[cfg(test)]
mod test {
    use near_sdk::{
        json_types::{U128, U64},
        serde_json::to_string,
        AccountId,
    };

    use crate::{
        common::U32,
        jar::view::{AggregatedTokenAmountView, JarView},
    };

    #[test]
    fn serialize_jar_views() {
        let jar_vew = JarView {
            index: U32(1),
            account_id: AccountId::new_unchecked("aaa".to_string()),
            product_id: "aaa".to_string(),
            created_at: U64(2),
            principal: U128(3),
            claimed_balance: U128(4),
            is_penalty_applied: false,
        };

        dbg!(&jar_vew);

        assert_eq!(
            to_string(&jar_vew).unwrap(),
            r#"{"index":"1","account_id":"aaa","product_id":"aaa","created_at":"2","principal":"3","claimed_balance":"4","is_penalty_applied":false}"#
        );

        let amount_view = AggregatedTokenAmountView {
            detailed: [(U32(0), U128(1_000_000))].into(),
            total: U128(1_000_000),
        };

        println!("{}", to_string(&amount_view).unwrap());

        dbg!(&amount_view);

        assert_eq!(
            to_string(&amount_view).unwrap(),
            r#"{"detailed":{"0":"1000000"},"total":"1000000"}"#
        );
    }
}
