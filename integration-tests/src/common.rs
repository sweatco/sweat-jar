use std::process::{Command, Stdio};

use serde_json::Value;

pub trait ValueGetters {
    fn get_u128(&self, key: &str) -> u128;
    fn get_interest(&self) -> u128;
}

impl ValueGetters for Value {
    fn get_u128(&self, key: &str) -> u128 {
        self.as_object()
            .unwrap()
            .get(key)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
            .parse::<u128>()
            .unwrap()
    }

    fn get_interest(&self) -> u128 {
        self.as_object().unwrap().get("amount").unwrap().get_u128("total")
    }
}

/// Compile contract in release mode and prepare it for integration tests usage
#[test]
pub fn build_contract() -> anyhow::Result<()> {
    Command::new("make")
        .arg("build")
        .current_dir("..")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    Ok(())
}
