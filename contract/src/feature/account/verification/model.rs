use near_sdk::{env, env::panic_str, json_types::Base64VecU8, require, AccountId};
use sweat_jar_model::{
    data::{
        deposit::{AirdropMessage, DepositMessage, DepositTicket, Purpose},
        product::ProductModelApi,
    },
    signer::MessageVerifier,
    TokenAmount, MS_IN_DAY,
};

use crate::Contract;

impl Contract {
    pub(crate) fn verify(
        &self,
        purpose: Purpose,
        account_id: &AccountId,
        amount: TokenAmount,
        ticket: &DepositTicket,
        signature: Option<&Base64VecU8>,
    ) {
        let product = self.get_product(&ticket.product_id);

        if let Some(pk) = &product.get_public_key() {
            let Some(signature) = signature else {
                panic_str("Signature is required");
            };
            ticket.verify_expiration_date();

            let account = self.try_get_account(account_id);
            let nonce = account.map_or(0, |account| account.nonce);
            let message = DepositMessage::new(
                purpose,
                &env::current_account_id(),
                account_id,
                &ticket.product_id,
                amount,
                ticket.valid_until.0,
                nonce,
            );

            MessageVerifier::new(pk).verify(message.material(), &message.sha256(), &signature.0);
        }
    }

    pub(crate) fn verify_airdrop(
        &self,
        ticket: &DepositTicket,
        amount: TokenAmount,
        receivers: &[AccountId],
        signature: Option<&Base64VecU8>,
    ) {
        let product = self.get_product(&ticket.product_id);

        if let Some(pk) = &product.get_public_key() {
            let Some(signature) = signature else {
                panic_str("Signature is required");
            };
            ticket.verify_expiration_date();

            let message = AirdropMessage::new(
                &env::current_account_id(),
                &ticket.product_id,
                amount,
                receivers,
                ticket.valid_until.0,
            );

            MessageVerifier::new(pk).verify(message.material(), &message.sha256(), &signature.0);
        }
    }
}

/// Signer is trusted to issue short-lived tickets (backend uses ~2-minute
/// windows), but nothing on-chain capped how far in the future `valid_until`
/// could be. Bounding it here caps the blast radius of a compromised/buggy
/// signer to at most this window rather than an unlimited one.
const MAX_TICKET_LIFETIME_MS: u64 = 7 * MS_IN_DAY;

trait JarTicketVerifier {
    fn verify_expiration_date(&self);
}

impl JarTicketVerifier for DepositTicket {
    fn verify_expiration_date(&self) {
        let now = env::block_timestamp_ms();
        require!(now <= self.valid_until.0, "Ticket is outdated");
        require!(
            self.valid_until.0 <= now + MAX_TICKET_LIFETIME_MS,
            "Ticket valid_until is too far in the future"
        );
    }
}
