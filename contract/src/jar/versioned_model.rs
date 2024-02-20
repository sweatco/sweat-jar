use std::ops::{Deref, DerefMut};

use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId,
};
use sweat_jar_model::{jar::JarId, ProductId, TokenAmount};

use crate::{
    common::Timestamp,
    jar::{model::JarCache, model_v2::JarV2},
    product::model::Product,
};

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
pub enum Jar {
    V2(JarV2),
}

impl Jar {
    pub fn create(
        id: JarId,
        account_id: AccountId,
        product_id: ProductId,
        principal: TokenAmount,
        created_at: Timestamp,
    ) -> Self {
        JarV2 {
            id,
            account_id,
            product_id,
            principal,
            created_at,
            cache: None,
            claimed_balance: 0,
            is_pending_withdraw: false,
            is_penalty_applied: false,
            claim_remainder: 0,
        }
        .into()
    }

    pub fn unlocked(&self) -> Self {
        JarV2 {
            is_pending_withdraw: false,
            ..self.inner()
        }
        .into()
    }

    pub fn with_id(mut self, id: JarId) -> Self {
        self.id = id;
        self
    }

    pub fn withdrawn(&self, product: &Product, withdrawn_amount: TokenAmount, now: Timestamp) -> Self {
        JarV2 {
            principal: self.principal - withdrawn_amount,
            cache: Some(JarCache {
                updated_at: now,
                interest: self.get_interest(product, now).0,
            }),
            ..self.inner().clone()
        }
        .into()
    }

    fn inner(&self) -> JarV2 {
        match self {
            Self::V2(jar) => jar.clone(),
        }
    }
}

impl Deref for Jar {
    type Target = JarV2;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V2(jar) => jar,
        }
    }
}

impl DerefMut for Jar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V2(jar) => jar,
        }
    }
}

impl From<JarV2> for Jar {
    fn from(value: JarV2) -> Self {
        Self::V2(value)
    }
}
