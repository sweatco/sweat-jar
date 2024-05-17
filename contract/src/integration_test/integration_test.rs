#![cfg(feature = "integration-test")]

use near_sdk::{env, near_bindgen, AccountId, Timestamp};
use sweat_jar_model::{api::IntegrationTestMethods, jar::JarView, ProductId};

use crate::{jar::model::Jar, Contract, ContractExt};

#[near_bindgen]
impl IntegrationTestMethods for Contract {
    #[mutants::skip]
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }

    #[mutants::skip]
    fn bulk_create_jars(
        &mut self,
        account_id: AccountId,
        product_id: ProductId,
        principal: u128,
        number_of_jars: u16,
    ) -> Vec<JarView> {
        self.assert_manager();
        (0..number_of_jars)
            .map(|_| self.create_jar_for_integration_tests(&account_id, &product_id, principal))
            .collect()
    }
}

impl Contract {
    fn create_jar_for_integration_tests(
        &mut self,
        account_id: &AccountId,
        product_id: &ProductId,
        amount: u128,
    ) -> JarView {
        let product = self.get_product(&product_id);

        product.assert_enabled();
        product.assert_cap(amount);

        let id = self.increment_and_get_last_jar_id();
        let now = env::block_timestamp_ms();
        let jar = Jar::create(id, account_id.clone(), product_id.clone(), amount, now);

        self.add_new_jar(account_id, jar.clone());

        jar.into()
    }
}
