use near_sdk::{json_types::Base64VecU8, near, AccountId};
use sweat_jar_model::{jar::JarTicket, product::Terms, TokenAmount};

use crate::{
    common::Timestamp,
    event::{emit, EventKind::Deposit},
    product::model::v1::ProductAssertions,
    Contract,
};

/// A cached value that stores calculated interest based on the current state of the jar.
/// This cache is updated whenever properties that impact interest calculation change,
/// allowing for efficient interest calculations between state changes.
#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct JarCache {
    pub updated_at: Timestamp,
    pub interest: TokenAmount,
}

impl Contract {
    pub(crate) fn deposit(
        &mut self,
        account_id: AccountId,
        ticket: JarTicket,
        amount: TokenAmount,
        signature: &Option<Base64VecU8>,
    ) {
        self.assert_migrated(&account_id);

        let product_id = &ticket.product_id;
        let product = self.get_product(product_id);

        product.assert_enabled();
        product.assert_cap(amount);
        self.verify(&account_id, amount, &ticket, signature);

        let account = self.get_or_create_account_mut(&account_id);

        if signature.is_some() {
            account.nonce += 1;
        }

        if matches!(product.terms, Terms::ScoreBased(_)) {
            account.try_set_timezone(ticket.timezone);
        }

        account.deposit(product_id, amount, None);

        emit(Deposit((product_id.clone(), amount.into())));
    }
}
