// Each test file under `tests/` is compiled as its own binary and pulls this
// module in via `mod common;`, so not every helper is used by every binary.
#![allow(dead_code)]

pub mod ft;
pub mod interest;
pub mod jar;
pub mod panic;
pub mod prepare;
pub mod product;

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;

pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key: SigningKey = SigningKey::generate(&mut OsRng);
    let verifying_key: VerifyingKey = VerifyingKey::from(&signing_key);

    (signing_key, verifying_key)
}
