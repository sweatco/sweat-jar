use std::collections::HashMap;

use near_sdk::near;

use crate::{
    data::{
        booster::BoosterIndex,
        jar::{Jar, JarCompanion},
        product::ProductId,
        score::AccountScore,
    },
    DurationDays,
};

use super::{
    features::{Feature, Features},
    v1::AccountV1,
};

#[near]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountV2 {
    /// TODO: doc change for BE migration
    pub nonce: u32,
    pub jars: HashMap<ProductId, Jar>,
    pub score: AccountScore,
    pub features: Features,
    pub booster: Option<AppliedBooster>,
}

#[near]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AppliedBooster {
    pub index: BoosterIndex,
    pub applied_at: DurationDays,
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq)]
pub struct AccountV2Companion {
    pub nonce: Option<u32>,
    pub jars: Option<HashMap<ProductId, JarCompanion>>,
    pub score: Option<AccountScore>,
    pub features: Option<Features>,
}

impl From<AccountV1> for AccountV2 {
    fn from(value: AccountV1) -> Self {
        let mut features = Features::new();
        features.set_feature_enabled(&Feature::IncreasedApy, !value.is_penalty_applied);

        Self {
            nonce: value.nonce,
            jars: value.jars,
            score: value.score,
            features,
            booster: None,
        }
    }
}
