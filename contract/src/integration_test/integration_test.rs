use near_sdk::{env, near_bindgen, AccountId, Timestamp};
use sweat_jar_model::ProductId;

use crate::{
    jar::{account::v1::AccountV1, model::Jar},
    Contract, ContractExt,
};

#[allow(dead_code)]
pub trait IntegrationTestMethods {
    fn block_timestamp_ms(&self) -> Timestamp;
    fn bulk_create_jars(&mut self, account_id: AccountId, product_id: ProductId, principal: u128, number_of_jars: u16);
    fn get_user_account(&self, account_id: AccountId) -> &AccountV1;
}

#[mutants::skip]
#[near_bindgen]
impl IntegrationTestMethods for Contract {
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }

    fn bulk_create_jars(&mut self, account_id: AccountId, product_id: ProductId, principal: u128, number_of_jars: u16) {
        self.assert_manager();
        let now = env::block_timestamp_ms();
        (0..number_of_jars)
            .for_each(|_| self.create_jar_for_integration_tests(&account_id, &product_id, principal, now));
    }

    fn get_user_account(&self, account_id: AccountId) -> &AccountV1 {
        self.accounts.get(&account_id).unwrap()
    }
}

#[mutants::skip]
impl Contract {
    fn create_jar_for_integration_tests(
        &mut self,
        account_id: &AccountId,
        product_id: &ProductId,
        amount: u128,
        now: u64,
    ) {
        let id = self.increment_and_get_last_jar_id();
        let jar = Jar::create(id, account_id.clone(), product_id.clone(), amount, now);

        self.add_new_jar(account_id, jar);
    }
}
