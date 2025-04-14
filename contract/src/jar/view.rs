use sweat_jar_model::{
    data::{
        jar::{Jar, JarView},
        product::ProductId,
    },
    Timestamp,
};

pub(crate) struct DetailedJar(pub(crate) ProductId, pub(crate) Jar);

impl From<&DetailedJar> for Vec<JarView> {
    fn from(value: &DetailedJar) -> Self {
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
    format!("{product_id}_{created_at}")
}
