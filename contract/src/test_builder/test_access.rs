use near_sdk::test_utils::test_env::alice;
use sweat_jar_model::{
    api::{JarApi, StepsApi},
    Steps,
};

use crate::{common::tests::Context, product::model::Product};

pub(crate) trait TestAccess {
    fn product(&self, id: &str) -> Product;
    fn interest(&self, id: u32) -> u128;
    fn record_steps(&mut self, timestamp: u64, steps: Steps);
}

impl TestAccess for Context {
    fn product(&self, id: &str) -> Product {
        self.contract().get_product(&id.to_string())
    }

    fn interest(&self, id: u32) -> u128 {
        self.contract().get_interest(vec![id.into()], alice()).amount.total.0
    }

    fn record_steps(&mut self, timestamp: u64, steps: Steps) {
        self.contract().record_steps(timestamp.into(), vec![(alice(), steps)])
    }
}
