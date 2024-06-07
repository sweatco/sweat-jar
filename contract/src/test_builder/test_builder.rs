#![cfg(test)]

use near_sdk::NearToken;
use sweat_jar_model::jar::JarId;

use crate::{
    common::tests::Context,
    jar::model::Jar,
    product::model::Product,
    test_builder::ProductBuilder,
    test_utils::{admin, PRODUCT},
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
    /// Add default product with APY
    pub fn product(mut self, apy: u32) -> Self {
        self.products.push(Product::new().id(PRODUCT).apy(apy));
        self
    }

    /// Build and add custom product
    pub fn product_build(mut self, id: &'static str, builder: impl ProductBuilder) -> Self {
        self.products.push(builder.build(id));
        self
    }

    /// Add default jar for `Alice` with 100 tokens and with last added to builder `Product`
    pub fn jar(mut self, id: JarId) -> Self {
        let product_id = &self.products.last().expect("Create product first").id;
        self.jars.push(
            Jar::new(id)
                .product_id(product_id)
                .principal(NearToken::from_near(100).as_yoctonear()),
        );
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
