use near_sdk::AccountId;
use sweat_jar_model::{
    api::{ClaimApi, JarApi, ScoreApi},
    jar::JarId,
    Score,
};

use crate::{common::tests::Context, product::model::Product, test_utils::UnwrapPromise};

pub(crate) trait TestAccess {
    fn product(&self, id: &str) -> Product;
    fn interest(&self, id: JarId, account_id: AccountId) -> u128;
    fn record_score(&mut self, timestamp: u64, score: Score, account_id: AccountId);
    fn claim_total(&mut self, account_id: AccountId) -> u128;
    fn claim_jar(&mut self, account_id: AccountId, jar_id: JarId) -> u128;
}

impl TestAccess for Context {
    fn product(&self, id: &str) -> Product {
        self.contract().get_product(&id.to_string())
    }

    fn interest(&self, id: JarId, account_id: AccountId) -> u128 {
        self.contract().get_interest(vec![id.into()], account_id).amount.total.0
    }

    fn record_score(&mut self, timestamp: u64, score: Score, account_id: AccountId) {
        self.contract()
            .record_score(timestamp.into(), vec![(account_id, score)])
    }

    fn claim_total(&mut self, account_id: AccountId) -> u128 {
        self.switch_account(account_id);
        self.contract().claim_total(None).unwrap().get_total().0
    }

    fn claim_jar(&mut self, account_id: AccountId, jar_id: JarId) -> u128 {
        self.switch_account(account_id);
        self.contract()
            .claim_jars(vec![jar_id.into()], None, None)
            .unwrap()
            .get_total()
            .0
    }
}
