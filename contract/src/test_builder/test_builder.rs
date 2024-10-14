// #![cfg(test)]
//
// use sweat_jar_model::jar::JarId;
//
// use crate::{
//     common::tests::Context,
//     jar::model::Jar,
//     product::model::Product,
//     score::AccountScore,
//     test_builder::{jar_builder::JarBuilder, ProductBuilder},
//     test_utils::admin,
// };
//
// pub(crate) struct TestBuilder {
//     context: Context,
//     products: Vec<Product>,
//     jars: Vec<Jar>,
// }
//
// impl TestBuilder {
//     pub fn new() -> Self {
//         Self {
//             context: Context::new(admin()),
//             products: vec![],
//             jars: vec![],
//         }
//     }
// }
//
// impl TestBuilder {
//     /// Build and add custom product
//     pub fn product(mut self, id: &'static str, builder: impl ProductBuilder) -> Self {
//         self.products.push(builder.build(id));
//         self
//     }
//
//     /// Build and add custom jar
//     pub fn jar(mut self, id: JarId, builder: impl JarBuilder) -> Self {
//         let product = self.products.last().expect("Create product first");
//         let product_id = &product.id;
//
//         let jar = builder.build(id, product_id, 100);
//
//         let account_id = &jar.account_id;
//
//         if product.is_score_product() {
//             if self.context.contract().get_score(account_id).is_none() {
//                 let Some(timezone) = builder.timezone() else {
//                     panic!("Step jar without timezone");
//                 };
//
//                 self.context.contract().accounts.entry(account_id.clone()).or_default();
//
//                 self.context.contract().accounts.get_mut(account_id).unwrap().score = AccountScore::new(timezone);
//             }
//         }
//
//         self.jars.push(jar);
//         self
//     }
// }
//
// impl TestBuilder {
//     pub fn build(self) -> Context {
//         self.context.with_products(&self.products).with_jars(&self.jars)
//     }
// }
