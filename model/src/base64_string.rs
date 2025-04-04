use std::ops::Deref;

use base64::{engine::general_purpose::STANDARD, Engine};

pub struct Base64String(String);

impl From<Vec<u8>> for Base64String {
    fn from(value: Vec<u8>) -> Self {
        Self(STANDARD.encode(value))
    }
}

impl From<Base64String> for Vec<u8> {
    fn from(value: Base64String) -> Self {
        STANDARD.decode(value.0).expect("Unable to decode Base64 string")
    }
}

impl Deref for Base64String {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
