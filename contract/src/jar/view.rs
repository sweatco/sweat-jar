use near_sdk::json_types::{U128, U64};
use sweat_jar_model::{jar::JarView, ProductId, U32};

use crate::jar::model::{Jar, JarV2};

impl From<Jar> for JarView {
    fn from(value: Jar) -> Self {
        Self {
            id: value.id.into(),
            product_id: value.product_id.clone(),
            created_at: U64(value.created_at),
            principal: U128(value.principal),
        }
    }
}

impl From<&Jar> for JarView {
    fn from(value: &Jar) -> Self {
        Self {
            id: value.id.into(),
            product_id: value.product_id.clone(),
            created_at: U64(value.created_at),
            principal: U128(value.principal),
        }
    }
}

pub struct DetailedJarV2(ProductId, JarV2);

impl From<&DetailedJarV2> for Vec<JarView> {
    fn from(value: &DetailedJarV2) -> Self {
        let product_id = value.0.clone();
        value
            .1
            .deposits
            .iter()
            .map(|deposit| JarView {
                product_id,
                id: format!("{product_id}_{}", deposit.created_at),
                created_at: deposit.created_at.into(),
                principal: deposit.principal.into(),
            })
            .collect()
    }
}
