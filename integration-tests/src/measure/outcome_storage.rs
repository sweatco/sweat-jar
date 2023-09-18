use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Mutex, MutexGuard},
};

use itertools::Itertools;
use workspaces::{
    result::{ExecutionOutcome, ExecutionSuccess},
    types::Gas,
    Account,
};

type Map = BTreeMap<String, ExecutionSuccess>;

static STORAGE: OutcomeStorage = OutcomeStorage {
    measuring: Mutex::new(vec![]),
    data: Mutex::new(Map::new()),
};

pub struct OutcomeStorage {
    measuring: Mutex<Vec<String>>,
    data: Mutex<Map>,
}

impl OutcomeStorage {
    fn get_measuring() -> MutexGuard<'static, Vec<String>> {
        STORAGE.measuring.lock().unwrap()
    }

    fn get_data() -> MutexGuard<'static, Map> {
        STORAGE.data.lock().unwrap()
    }

    fn start_measuring(account: &Account) {
        let mut measuring = Self::get_measuring();
        assert!(measuring.iter().find(|a| a == &account.id().as_str()).is_none());
        measuring.push(account.id().to_string());
    }

    fn stop_measuring(account: &Account) {
        let mut measuring = Self::get_measuring();

        let index = measuring
            .iter()
            .find_position(|a| a == &account.id().as_str())
            .unwrap()
            .0;
        measuring.remove(index);
    }

    /// Execute command and measure its gas price
    pub async fn measure<Output>(
        label: &str,
        account: &Account,
        future: impl Future<Output = anyhow::Result<Output>>,
    ) -> anyhow::Result<(Gas, Output)> {
        Self::start_measuring(account);
        let output = future.await?;
        Self::stop_measuring(account);

        let result = OutcomeStorage::get_result(&account, label);

        Ok((result.gas_burnt, output))
    }
}

impl OutcomeStorage {
    /// Store successful execution result
    pub fn add_result(result: ExecutionSuccess) {
        let executon = result.outcome().executor_id.clone();

        if !Self::get_measuring().contains(&executon.to_string()) {
            return;
        }

        let existing = Self::get_data().insert(executon.to_string(), result);
        assert!(existing.is_none());
    }

    /// Get execution result for given manager account
    pub fn get_result(account: &Account, label: &str) -> ExecutionOutcome {
        Self::get_data()
            .get(account.id().as_str())
            .unwrap()
            .outcomes()
            .into_iter()
            .find(|outcome| outcome.logs.iter().find(|log| log.contains(label)).is_some())
            .unwrap()
            .clone()
    }
}
