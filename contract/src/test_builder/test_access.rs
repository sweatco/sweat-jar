use near_sdk::AccountId;
use sweat_jar_model::{
    api::{ClaimApi, JarApi, ScoreApi},
    jar::JarId,
    Score,
};

use crate::{common::tests::Context, jar::model::Jar, product::model::Product, test_utils::UnwrapPromise};

pub trait TestAccess {
    fn _product(&self, id: &str) -> Product;
    fn interest(&self, id: JarId) -> u128;
    fn record_score(&mut self, timestamp: u64, score: Score, account_id: AccountId);
    fn claim_total(&mut self, account_id: AccountId) -> u128;
    fn jar(&self, id: JarId) -> Jar;
    fn jar_account_for_id(&self, id: JarId) -> AccountId;
}

impl TestAccess for Context {
    fn _product(&self, id: &str) -> Product {
        self.contract().get_product(&id.to_string())
    }

    fn interest(&self, id: JarId) -> u128 {
        let account_id = self.jar_account_for_id(id);
        self.contract().get_interest(vec![id.into()], account_id).amount.total.0
    }

    fn record_score(&mut self, timestamp: u64, score: Score, account_id: AccountId) {
        self.contract()
            .record_score(vec![(account_id, vec![(score, timestamp.into())])])
    }

    fn claim_total(&mut self, account_id: AccountId) -> u128 {
        self.switch_account(account_id);
        self.contract().claim_total(None).unwrap().get_total().0
    }

    fn jar(&self, id: JarId) -> Jar {
        let account_id = self.jar_account_for_id(id);
        self.contract().get_jar_internal(&account_id, id)
    }

    fn jar_account_for_id(&self, id: JarId) -> AccountId {
        for (account, jars) in &self.account_jars {
            if jars.contains(&id) {
                return account.clone();
            }
        }

        panic!("Accound for jar id: {id} not found")
    }
}
