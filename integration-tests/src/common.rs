use serde_json::Value;

pub trait ValueGetters {
    fn get_u128(&self, key: &str) -> u128;
}

impl ValueGetters for Value {
    fn get_u128(&self, key: &str) -> u128 {
        self.as_object().unwrap().get(key).unwrap().as_str().unwrap().to_string().parse::<u128>().unwrap()
    }
}