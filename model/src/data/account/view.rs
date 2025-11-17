use std::collections::HashMap;

use near_sdk::near;
use strum::IntoEnumIterator;

use super::Account;
use crate::{
    data::{account::features::Feature, jar::JarView, product::ProductId},
    AccountScoreView, Timezone,
};

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountView {
    pub nonce: u32,
    pub jars: HashMap<ProductId, JarView>,
    pub score: AccountScoreView,
    pub is_penalty_applied: bool,
    pub features: HashMap<Feature, bool>,
    pub timezone: Timezone,
}

impl From<Account> for AccountView {
    fn from(value: Account) -> Self {
        AccountView {
            nonce: value.nonce,
            jars: value
                .jars
                .into_iter()
                .map(|(product_id, jar)| (product_id, jar.into()))
                .collect(),
            score: value.score.into(),
            is_penalty_applied: !value.features.is_feature_enabled(&Feature::IncreasedApy),
            features: Feature::iter()
                .map(|feature| (feature, value.features.is_feature_enabled(&feature)))
                .collect(),
            timezone: value.timezone,
        }
    }
}
