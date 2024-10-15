use std::ops::{Deref, DerefMut};

use near_sdk::near;
use sweat_jar_model::jar::JarId;

use crate::{
    jar::model::{AccountJarsLegacy, Jar},
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
    score::AccountScore,
};

#[near]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountV1 {
    /// The last jar ID. Is used as nonce in `get_ticket_hash` method.
    pub last_id: JarId,
    pub jars: Vec<Jar>,
    pub score: AccountScore,
}

impl Deref for AccountV1 {
    type Target = Vec<Jar>;

    fn deref(&self) -> &Self::Target {
        &self.jars
    }
}

impl DerefMut for AccountV1 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.jars
    }
}

impl From<AccountJarsLegacy> for AccountV1 {
    fn from(value: AccountJarsLegacy) -> Self {
        Self {
            last_id: value.last_id,
            jars: value.jars.into_iter().map(Into::into).collect(),
            score: AccountScore::default(),
        }
    }
}

impl From<AccountJarsNonVersioned> for AccountV1 {
    fn from(value: AccountJarsNonVersioned) -> Self {
        Self {
            last_id: value.last_id,
            jars: value.jars,
            score: AccountScore::default(),
        }
    }
}
