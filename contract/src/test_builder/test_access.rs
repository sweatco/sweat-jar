use near_sdk::AccountId;
use sweat_jar_model::{
    api::{JarApi, StepsApi},
    jar::JarId,
    Steps,
};

use crate::{common::tests::Context, product::model::Product};

pub(crate) trait TestAccess {
    fn product(&self, id: &str) -> Product;
    fn interest(&self, id: JarId, account_id: AccountId) -> u128;
    fn record_steps(&mut self, timestamp: u64, steps: Steps, account_id: AccountId);
}

impl TestAccess for Context {
    fn product(&self, id: &str) -> Product {
        self.contract().get_product(&id.to_string())
    }

    fn interest(&self, id: JarId, account_id: AccountId) -> u128 {
        self.contract().get_interest(vec![id.into()], account_id).amount.total.0
    }

    fn record_steps(&mut self, timestamp: u64, steps: Steps, account_id: AccountId) {
        self.contract()
            .record_steps(timestamp.into(), vec![(account_id, steps)])
    }
}
