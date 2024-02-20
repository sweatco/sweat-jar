#![cfg(feature = "integration-test")]

use near_sdk::{env, near_bindgen, AccountId, Timestamp};
use sweat_jar_model::{api::IntegrationTestMethods, ProductId, TokenAmount};

use crate::{Contract, ContractExt, Jar};

#[near_bindgen]
impl IntegrationTestMethods for Contract {
    #[mutants::skip]
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }

    #[mutants::skip]
    fn bulk_create_jars(
        &mut self,
        accounts: Vec<AccountId>,
        product_id: ProductId,
        locked_amount: TokenAmount,
        jars_count: u32,
    ) {
        let now = env::block_timestamp_ms();

        for account in accounts {
            let jar = Jar::create(0, account.clone(), product_id.clone(), locked_amount, now);

            let jars = self.account_jars.entry(account.clone()).or_default();

            jars.jars.reserve(jars_count as usize);

            for _ in 0..jars_count {
                self.last_jar_id += 1;

                jars.last_id = jar.id;
                jars.push(jar.clone().with_id(self.last_jar_id));
            }

            jars.last_id = jars_count - 1;
        }
    }
}
