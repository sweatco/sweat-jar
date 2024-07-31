#![cfg(test)]

use sweat_jar_model::jar::JarId;

use crate::{
    common::tests::Context,
    jar::model::Jar,
    product::model::Product,
    test_builder::{jar_builder::JarBuilder, ProductBuilder},
    test_utils::admin,
};

pub(crate) struct TestBuilder {
    products: Vec<Product>,
    jars: Vec<Jar>,
}

impl TestBuilder {
    pub fn new() -> Self {
        Self {
            products: vec![],
            jars: vec![],
        }
    }
}

impl TestBuilder {
    /// Build and add custom product
    pub fn product(mut self, id: &'static str, builder: impl ProductBuilder) -> Self {
        self.products.push(builder.build(id));
        self
    }

    /// Build and add custom jar
    pub fn jar(mut self, id: JarId, builder: impl JarBuilder) -> Self {
        let product_id = &self.products.last().expect("Create product first").id;
        self.jars.push(builder.build(id, product_id, 100));
        self
    }
}

impl TestBuilder {
    pub fn build(self) -> Context {
        Context::new(admin())
            .with_products(&self.products)
            .with_jars(&self.jars)
    }
}
