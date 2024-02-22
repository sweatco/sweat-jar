#![cfg(test)]

use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};

type ThreadId = String;
type ValueKey = String;
type Value = String;

type Map = BTreeMap<ThreadId, BTreeMap<ValueKey, Value>>;

/// This structure can store arbitrary data and link it to a particular thread.
/// It allows the data to not be mixed in multithreaded test environment.
struct TestDataStorage {
    data: Mutex<Map>,
}

static DATA: TestDataStorage = TestDataStorage {
    data: Mutex::new(BTreeMap::new()),
};

const FUTURE_SUCCESS_KEY: &str = "FUTURE_SUCCESS_KEY";
const LOG_EVENTS_KEY: &str = "LOG_EVENTS_KEY";

fn data() -> MutexGuard<'static, Map> {
    DATA.data.lock().unwrap()
}

pub(crate) fn set_test_future_success(success: bool) {
    let mut data = data();
    let map = data.entry(thread_name()).or_default();
    map.insert(FUTURE_SUCCESS_KEY.to_owned(), success.to_string());
}

pub(crate) fn get_test_future_success() -> bool {
    let data = data();

    let Some(map) = data.get(&thread_name()) else {
        return true;
    };

    let Some(value) = map.get(FUTURE_SUCCESS_KEY) else {
        return true;
    };

    value.parse().unwrap()
}

#[mutants::skip]
pub(crate) fn set_test_log_events(enabled: bool) {
    let mut data = data();
    let map = data.entry(thread_name()).or_default();
    map.insert(LOG_EVENTS_KEY.to_owned(), enabled.to_string());
}

#[mutants::skip]
pub(crate) fn should_log_events() -> bool {
    let data = data();

    let Some(map) = data.get(&thread_name()) else {
        return true;
    };

    let Some(value) = map.get(LOG_EVENTS_KEY) else {
        return true;
    };

    value.parse().unwrap()
}

fn thread_name() -> String {
    std::thread::current().name().unwrap().to_owned()
}

#[test]
fn thread_name_test() {
    assert_eq!(thread_name(), "common::test_data::thread_name_test");
}

#[test]
fn test_data_storage() {
    assert_eq!(get_test_future_success(), true);
    set_test_future_success(false);
    assert_eq!(get_test_future_success(), false);
    set_test_future_success(true);
    assert_eq!(get_test_future_success(), true)
}
