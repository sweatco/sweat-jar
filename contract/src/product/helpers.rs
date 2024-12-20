#![cfg(test)]

use base64::{engine::general_purpose, Engine};
use crypto_hash::{digest, Algorithm};
use ed25519_dalek::{Signer, SigningKey};
use general_purpose::STANDARD;
use near_sdk::AccountId;
use rand::rngs::OsRng;
use sweat_jar_model::TokenAmount;

use crate::{common::tests::Context, jar::model::JarTicket, Contract};

pub(crate) struct MessageSigner {
    signing_key: SigningKey,
}

impl MessageSigner {
    pub(crate) fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key: SigningKey = SigningKey::generate(&mut csprng);

        Self { signing_key }
    }

    pub(crate) fn sign(&self, message: &str) -> Vec<u8> {
        let message_hash = digest(Algorithm::SHA256, message.as_bytes());
        let signature = self.signing_key.sign(message_hash.as_slice());
        signature.to_bytes().to_vec()
    }

    pub(crate) fn sign_base64(&self, message: &str) -> String {
        STANDARD.encode(self.sign(message))
    }

    pub(crate) fn public_key(&self) -> Vec<u8> {
        self.signing_key.verifying_key().as_ref().to_vec()
    }
}

impl Context {
    pub(crate) fn get_signature_material(
        &self,
        receiver_id: &AccountId,
        ticket: &JarTicket,
        amount: TokenAmount,
    ) -> String {
        Contract::get_signature_material(
            &self.owner,
            receiver_id,
            &ticket.product_id,
            amount,
            ticket.valid_until.0,
            0,
        )
    }
}
