#![allow(dead_code)]

use std::{fs, path::PathBuf, process::Command};

pub(crate) fn load_wasm(wasm_path: &str) -> Vec<u8> {
    // Assuming that the Makefile is in root repository directory
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .unwrap();
    assert!(output.status.success(), "Failed to get Git repository root path");
    let git_root: PathBuf = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_string()
        .into();

    let wasm_filepath = fs::canonicalize(git_root.join(wasm_path)).expect("Failed to get wasm file path");
    fs::read(wasm_filepath).expect("Failed to load wasm")
}
