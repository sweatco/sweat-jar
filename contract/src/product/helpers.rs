#![cfg(test)]

use near_sdk::AccountId;
use sweat_jar_model::{jar::DepositTicket, signer::DepositMessage, TokenAmount};

use crate::common::tests::Context;

impl Context {
    pub(crate) fn get_deposit_message(
        &self,
        receiver_id: &AccountId,
        ticket: &DepositTicket,
        amount: TokenAmount,
    ) -> String {
        DepositMessage::new(
            &self.owner,
            receiver_id,
            &ticket.product_id,
            amount,
            ticket.valid_until.0,
            0,
        )
        .to_string()
    }
}
