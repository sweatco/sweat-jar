use std::ops::{Deref, DerefMut};

use near_sdk::near;
use sweat_jar_model::jar::JarId;

use crate::{
    jar::model::{AccountJarsLegacy, Jar},
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
    score::AccountScore,
};

#[near]
#[derive(Default, Debug, PartialEq)]
pub struct AccountV2 {
    /// Is used as nonce in `get_ticket_hash` method.
    pub nonce: u32,
    pub jars: Vec<Jar>,
    pub score: AccountScore,
    pub is_penalty_applied: bool,
}
