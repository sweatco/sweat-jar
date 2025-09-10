use std::collections::HashMap;

use near_sdk::near;

use crate::{
    data::{jar::JarView, product::ProductId},
    AccountScore,
};

use super::Account;

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountView {
    pub nonce: u32,
    pub jars: HashMap<ProductId, JarView>,
    pub score: AccountScore,
    pub is_penalty_applied: bool,
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
            score: value.score,
            is_penalty_applied: value.is_penalty_applied,
        }
    }
}
