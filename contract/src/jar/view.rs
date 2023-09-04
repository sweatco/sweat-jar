use std::{collections::HashMap, fmt::Debug};

use near_sdk::{
    json_types::{U128, U64},
    serde::{Deserialize, Serialize},
    AccountId,
};

use crate::{common::U32, product::model::ProductId, *};

pub type JarIndexView = U32;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<JarIndexView, U128>,
    pub total: U128,
}

#[cfg(test)]
mod test {
    use near_sdk::{
        json_types::{U128, U64},
        AccountId,
    };

    use crate::{
        common::{tests::test_derived_macros, U32},
        jar::view::{AggregatedTokenAmountView, JarView},
    };

    #[test]
    fn jar_views_macros() {
        let jar_view = JarView {
            index: U32(1),
            account_id: AccountId::new_unchecked("aaa".to_string()),
            product_id: "aaa".to_string(),
            created_at: U64(2),
            principal: U128(3),
            claimed_balance: U128(4),
            is_penalty_applied: false,
        };

        test_derived_macros(&jar_view);

        let amount_view = AggregatedTokenAmountView {
            detailed: [(U32(0), U128(1_000_000))].into(),
            total: U128(1_000_000),
        };

        test_derived_macros(&amount_view);
    }
}
