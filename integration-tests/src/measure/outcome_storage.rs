use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};

use workspaces::{
    result::{ExecutionOutcome, ExecutionSuccess},
    Account,
};

type Map = BTreeMap<String, ExecutionSuccess>;

static STORAGE: OutcomeStorage = OutcomeStorage {
    data: Mutex::new(Map::new()),
};

pub struct OutcomeStorage {
    data: Mutex<Map>,
}

impl OutcomeStorage {
    fn get() -> MutexGuard<'static, Map> {
        STORAGE.data.lock().unwrap()
    }

    pub fn add_result(result: ExecutionSuccess) {
        let manager = result.outcome().executor_id.clone();
        let _existing = Self::get().insert(manager.to_string(), result);
        // assert!(existing.is_none());
    }

    /// Get execution result for given manager account
    pub fn get_result(manager: &Account, label: &str) -> ExecutionOutcome {
        Self::get()
            .get(manager.id().as_str())
            .unwrap()
            .outcomes()
            .into_iter()
            .find(|outcome| outcome.logs.iter().find(|log| log.contains(label)).is_some())
            .unwrap()
            .clone()
    }
}
