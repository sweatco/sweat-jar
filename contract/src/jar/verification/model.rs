use near_sdk::{env, env::panic_str, json_types::Base64VecU8, require, AccountId};
use sweat_jar_model::{
    jar::JarTicket,
    signer::{DepositMessage, MessageVerifier},
    TokenAmount,
};

use crate::{product::model::v1::ProductModelApi, Contract};

impl Contract {
    pub(crate) fn verify(
        &mut self,
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &JarTicket,
        signature: &Option<Base64VecU8>,
    ) {
        let account = self.try_get_account(account_id);
        let product = self.get_product(&ticket.product_id);

        if let Some(pk) = &product.get_public_key() {
            let Some(signature) = signature else {
                panic_str("Signature is required");
            };
            ticket.verify_expiration_date();

            let nonce = account.map_or(0, |account| account.nonce);
            let message = DepositMessage::new(
                &env::current_account_id(),
                account_id,
                &ticket.product_id,
                amount,
                ticket.valid_until.0,
                nonce,
            );

            MessageVerifier::new(pk).verify(&message.sha256(), &signature.0);
        }
    }
}

trait JarTicketVerifier {
    fn verify_expiration_date(&self);
}

impl JarTicketVerifier for JarTicket {
    fn verify_expiration_date(&self) {
        let is_time_valid = env::block_timestamp_ms() <= self.valid_until.0;
        require!(is_time_valid, "Ticket is outdated");
    }
}
