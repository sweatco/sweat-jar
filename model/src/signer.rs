use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

use ed25519_dalek::{Signature, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use near_sdk::require;
#[cfg(not(feature = "integration-test"))]
use near_sdk::AccountId;
#[cfg(feature = "integration-test")]
use nitka::near_sdk::AccountId;
use sha2::{Digest, Sha256};

use crate::{ProductId, Timestamp, TokenAmount};

pub struct DepositMessage(String);

pub fn sha256(value: &[u8]) -> Vec<u8> {
    Sha256::digest(value).to_vec()
}

impl DepositMessage {
    pub fn new(
        contract_account_id: &AccountId,
        receiver_account_id: &AccountId,
        product_id: &ProductId,
        amount: TokenAmount,
        valid_until: Timestamp,
        nonce: u32,
    ) -> Self {
        Self(format!(
            "{contract_account_id},{receiver_account_id},{product_id},{amount},{nonce},{valid_until}"
        ))
    }

    pub fn sha256(&self) -> Vec<u8> {
        sha256(self.0.as_bytes())
    }
}

impl Display for DepositMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.clone())
    }
}

impl Deref for DepositMessage {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
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

    pub fn verify(&self, message_sha256: &[u8], signature: &[u8]) {
        let signature = Signature::from_bytes(
            signature
                .try_into()
                .unwrap_or_else(|_| panic!("Signature must be {SIGNATURE_LENGTH} bytes")),
        );
        let is_signature_valid = self.verifying_key.verify_strict(message_sha256, &signature).is_ok();

        require!(is_signature_valid, "Not matching signature");
    }
}

#[cfg(feature = "testing")]
pub mod test_utils {
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
}
