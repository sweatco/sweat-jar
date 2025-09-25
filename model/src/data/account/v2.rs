use std::collections::HashMap;

use near_sdk::near;

use crate::data::{
    jar::{Jar, JarCompanion},
    product::ProductId,
    score::AccountScore,
};

use super::features::Features;

#[near]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountV2 {
    /// TODO: doc change for BE migration
    pub nonce: u32,
    pub jars: HashMap<ProductId, Jar>,
    pub score: AccountScore,
    pub features: Features,
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq)]
pub struct AccountV2Companion {
    pub nonce: Option<u32>,
    pub jars: Option<HashMap<ProductId, JarCompanion>>,
    pub score: Option<AccountScore>,
    pub features: Option<Features>,
}
