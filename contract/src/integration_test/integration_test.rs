#![cfg(feature = "integration-test")]

use near_sdk::{env, near_bindgen, AccountId, Timestamp};
use sweat_jar_model::{api::IntegrationTestMethods, ProductId, TokenAmount};

use crate::{
    jar::{account::v1::AccountV1, model::Deposit},
    Contract, ContractExt,
};

#[mutants::skip]
#[near_bindgen]
impl IntegrationTestMethods for Contract {
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }

    fn bulk_create_jars(&mut self, account_id: AccountId, product_id: ProductId, principal: u128, number_of_jars: u16) {
        self.assert_manager();
        let now = env::block_timestamp_ms();

        let account = self.get_or_create_account_mut(&account_id);
        for i in 0..number_of_jars {
            account.deposit_for_test(&product_id, now + i as u64, principal);
        }
    }
}

impl AccountV1 {
    fn deposit_for_test(&mut self, product_id: &ProductId, timestamp: Timestamp, principal: TokenAmount) {
        let deposit = Deposit::new(timestamp, principal);
        self.push(product_id, deposit);
    }
}
