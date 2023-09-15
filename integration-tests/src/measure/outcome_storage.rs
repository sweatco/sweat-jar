use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Mutex, MutexGuard},
};

use itertools::Itertools;
use workspaces::{
    result::{ExecutionOutcome, ExecutionSuccess},
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

    fn start_measuring(manager: &Account) {
        let mut measuring = Self::get_measuring();
        assert!(measuring.iter().find(|a| a == &manager.id().as_str()).is_none());
        measuring.push(manager.id().to_string());
    }

    fn stop_measuring(manager: &Account) {
        let mut measuring = Self::get_measuring();

        let index = measuring
            .iter()
            .find_position(|a| a == &manager.id().as_str())
            .unwrap()
            .0;
        measuring.remove(index);
    }

    pub async fn measure<Output>(manager: &Account, future: impl Future<Output = Output>) -> Output {
        Self::start_measuring(manager);
        let result = future.await;
        Self::stop_measuring(manager);
        result
    }
}

impl OutcomeStorage {
    pub fn add_result(result: ExecutionSuccess) {
        let manager = result.outcome().executor_id.clone();

        if !Self::get_measuring().contains(&manager.to_string()) {
            return;
        }

        let existing = Self::get_data().insert(manager.to_string(), result);
        assert!(existing.is_none());
    }

    /// Get execution result for given manager account
    pub fn get_result(manager: &Account, label: &str) -> ExecutionOutcome {
        Self::get_data()
            .get(manager.id().as_str())
            .unwrap()
            .outcomes()
            .into_iter()
            .find(|outcome| outcome.logs.iter().find(|log| log.contains(label)).is_some())
            .unwrap()
            .clone()
    }
}
