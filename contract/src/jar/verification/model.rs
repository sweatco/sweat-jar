use ed25519_dalek::{Signature, VerifyingKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use near_sdk::{
    env,
    env::{panic_str, sha256},
    json_types::Base64VecU8,
    require, AccountId,
};
use sweat_jar_model::{ProductId, TokenAmount};

use crate::{common::Timestamp, jar::model::JarTicket, Contract};

impl Contract {
    pub(crate) fn verify(
        &mut self,
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &JarTicket,
        signature: &Option<Base64VecU8>,
    ) {
        self.migrate_account_if_needed(account_id);

        let account = self.try_get_account(account_id);
        let product = self.get_product(&ticket.product_id);

        if let Some(pk) = &product.public_key {
            let Some(signature) = signature else {
                panic_str("Signature is required");
            };

            let is_time_valid = env::block_timestamp_ms() <= ticket.valid_until.0;
            require!(is_time_valid, "Ticket is outdated");

            let nonce = account.map_or(0, |account| account.nonce);
            let hash = Self::get_ticket_hash(account_id, amount, ticket, nonce);
            let is_signature_valid = Self::verify_signature(&signature.0, pk, &hash);

            require!(is_signature_valid, "Not matching signature");
        }
    }

    fn get_ticket_hash(account_id: &AccountId, amount: TokenAmount, ticket: &JarTicket, nonce: u32) -> Vec<u8> {
        sha256(
            Self::get_signature_material(
                &env::current_account_id(),
                account_id,
                &ticket.product_id,
                amount,
                ticket.valid_until.0,
                nonce,
            )
            .as_bytes(),
        )
    }

    pub(crate) fn get_signature_material(
        contract_account_id: &AccountId,
        receiver_account_id: &AccountId,
        product_id: &ProductId,
        amount: TokenAmount,
        valid_until: Timestamp,
        nonce: u32,
    ) -> String {
        format!(
            "{},{},{},{},{},{}",
            contract_account_id, receiver_account_id, product_id, amount, nonce, valid_until,
        )
    }

    fn verify_signature(signature: &[u8], product_public_key: &[u8], ticket_hash: &[u8]) -> bool {
        let signature_bytes: &[u8; SIGNATURE_LENGTH] = signature
            .try_into()
            .unwrap_or_else(|_| panic!("Signature must be {SIGNATURE_LENGTH} bytes"));

        let signature = Signature::from_bytes(signature_bytes);

        let public_key_bytes: &[u8; PUBLIC_KEY_LENGTH] = product_public_key
            .try_into()
            .unwrap_or_else(|_| panic!("Public key must be {PUBLIC_KEY_LENGTH} bytes"));

        VerifyingKey::from_bytes(public_key_bytes)
            .expect("Public key is invalid")
            .verify_strict(ticket_hash, &signature)
            .is_ok()
    }
}
