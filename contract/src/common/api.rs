#![cfg(feature = "integration-test")]

use near_sdk::{env, json_types::U128, near, AccountId, Timestamp};
use sweat_jar_model::{
    api::IntegrationTestMethods,
    data::product::{ProductAssertions, ProductId, Terms},
    Timezone,
};

use crate::{Contract, ContractExt};

#[mutants::skip]
#[near]
impl IntegrationTestMethods for Contract {
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }

    fn bulk_create_jars(&mut self, account_id: AccountId, product_id: ProductId, principal: u128, number_of_jars: u16) {
        self.assert_manager();
        let now = env::block_timestamp_ms();

        let account = self.get_or_create_account_mut(&account_id);
        for i in 0..number_of_jars {
            account.deposit(&product_id, principal, (now + i as u64).into());
        }
    }

    fn set_time_scale(&mut self, time_scale: f64) {
        self.assert_manager();
        self.time_scale = time_scale;
        sweat_jar_model::set_global_time_scale(time_scale);
    }

    fn seed_accounts(
        &mut self,
        product_id: ProductId,
        accounts: Vec<(AccountId, U128, Timezone)>,
        deposit_timestamp_ms: u64,
    ) {
        self.assert_manager();
        let product = self.get_product(&product_id);
        product.assert_enabled();

        for (account_id, principal, timezone) in accounts {
            let principal_value = principal.0;
            product.assert_cap(principal_value);

            let account = self.get_or_create_account_mut(&account_id);
            if matches!(product.terms, Terms::ScoreBased(_) | Terms::TieredScoreBased(_)) {
                account.timezone = timezone;
                account.score = Default::default();
            }
            account.booster = None;
            account.jars.remove(&product_id);
            account.deposit(&product_id, principal_value, Some(deposit_timestamp_ms));

            self.update_jar_cache(&account_id, &product_id);
        }
    }
}
