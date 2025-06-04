use ed25519_dalek::{Signature, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use near_sdk::env::panic_str;
use sha2::{Digest, Sha256};

pub fn sha256(value: &[u8]) -> Vec<u8> {
    Sha256::digest(value).to_vec()
}

pub struct MessageVerifier {
    verifying_key: VerifyingKey,
}

impl MessageVerifier {
    pub fn new(verifying_key_bytes: &[u8]) -> Self {
        let verifying_key_bytes: [u8; PUBLIC_KEY_LENGTH] = verifying_key_bytes
            .try_into()
            .unwrap_or_else(|_| panic!("Public key must be {PUBLIC_KEY_LENGTH} bytes"));
        let verifying_key = VerifyingKey::from_bytes(&verifying_key_bytes).expect("Verifying key is invalid");

        Self { verifying_key }
    }

    pub fn verify(&self, material: &str, message_sha256: &[u8], signature: &[u8]) {
        let signature = Signature::from_bytes(
            signature
                .try_into()
                .unwrap_or_else(|_| panic!("Signature must be {SIGNATURE_LENGTH} bytes")),
        );
        let is_signature_valid = self.verifying_key.verify_strict(message_sha256, &signature).is_ok();

        if !is_signature_valid {
            panic_str(&format!("Not matching signature. Contract material: {material}"));
        }
    }
}

#[cfg(feature = "testing")]
pub mod test_utils {
    use std::ops::Deref;

    use base64::{engine::general_purpose::STANDARD, Engine};
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    use crate::signer::sha256;

    pub struct MessageSigner {
        signing_key: SigningKey,
    }

    impl MessageSigner {
        pub fn new() -> Self {
            let mut csprng = OsRng;
            let signing_key: SigningKey = SigningKey::generate(&mut csprng);

            Self { signing_key }
        }

        pub fn sign(&self, message: &str) -> Vec<u8> {
            let message_hash = sha256(message.as_bytes());
            let signature = self.signing_key.sign(message_hash.as_slice());
            signature.to_bytes().to_vec()
        }

        pub fn public_key(&self) -> Vec<u8> {
            self.signing_key.verifying_key().as_ref().to_vec()
        }
    }

    pub struct Base64String(String);

    impl From<Vec<u8>> for Base64String {
        fn from(value: Vec<u8>) -> Self {
            Self(STANDARD.encode(value))
        }
    }

    impl Deref for Base64String {
        type Target = String;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
