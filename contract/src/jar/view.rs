use near_sdk::json_types::{U128, U64};
use sweat_jar_model::{jar::JarView, ProductId};

use crate::{
    common::Timestamp,
    jar::model::{JarV2, JarVersionedLegacy},
};

impl From<JarVersionedLegacy> for JarView {
    fn from(value: JarVersionedLegacy) -> Self {
        Self {
            id: value.id.to_string(),
            product_id: value.product_id.clone(),
            created_at: U64(value.created_at),
            principal: U128(value.principal),
        }
    }
}

impl From<&JarVersionedLegacy> for JarView {
    fn from(value: &JarVersionedLegacy) -> Self {
        Self {
            id: value.id.to_string(),
            product_id: value.product_id.clone(),
            created_at: U64(value.created_at),
            principal: U128(value.principal),
        }
    }
}

pub(crate) struct DetailedJarV2(pub(crate) ProductId, pub(crate) JarV2);

impl From<&DetailedJarV2> for Vec<JarView> {
    fn from(value: &DetailedJarV2) -> Self {
        let product_id = value.0.clone();
        value
            .1
            .deposits
            .iter()
            .map(|deposit| JarView {
                id: create_synthetic_jar_id(product_id.clone(), deposit.created_at),
                product_id: product_id.clone(),
                created_at: deposit.created_at.into(),
                principal: deposit.principal.into(),
            })
            .collect()
    }
}

pub fn create_synthetic_jar_id(product_id: ProductId, created_at: Timestamp) -> String {
    format!("{}_{}", product_id.clone(), created_at)
}
