use std::collections::HashMap;

use near_sdk::near;

use crate::data::{
    jar::{Jar, JarCompanion},
    product::ProductId,
    score::AccountScore,
};

#[near]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountV1 {
    /// TODO: doc change for BE migration
    pub nonce: u32,
    pub jars: HashMap<ProductId, Jar>,
    pub score: AccountScore,
    pub is_penalty_applied: bool,
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq)]
pub struct AccountV1Companion {
    pub nonce: Option<u32>,
    pub jars: Option<HashMap<ProductId, JarCompanion>>,
    pub score: Option<AccountScore>,
    pub is_penalty_applied: Option<bool>,
}

impl AccountV1 {
    #[must_use]
    pub fn with_sorted_deposits(&self) -> Self {
        let mut jars = self.jars.clone();
        for jar in jars.values_mut() {
            jar.sort_deposits();
        }

        Self { jars, ..self.clone() }
    }

    #[must_use]
    pub fn with_merged_deposits(&self) -> Self {
        let mut jars = self.jars.clone();
        for jar in jars.values_mut() {
            jar.merge_deposits();
            jar.sort_deposits();
        }

        Self { jars, ..self.clone() }
    }
}
