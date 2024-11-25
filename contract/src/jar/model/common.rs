use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    near, AccountId,
};
use sweat_jar_model::{Timezone, TokenAmount};

use crate::{common::Timestamp, product::model::v1::Terms, Contract};

/// The `JarTicket` struct represents a request to create a deposit jar for a corresponding product.
///
/// The data from this `JarTicket` is later combined with additional data, including the contract
/// account address, the recipient's account ID, the desired amount of tokens to deposit,
/// and the ID of the last jar created for the recipient. The concatenation of this data
/// forms a message that is then hashed using the SHA-256 algorithm. This resulting hash is used
/// to verify the authenticity of the data against an Ed25519 signature provided in the `ft_transfer_call` data.
#[derive(Clone, Debug)]
#[near(serializers=[json])]
pub struct JarTicket {
    /// The unique identifier of the product for which the jar is intended to be created.
    /// This `product_id` links the request to the specific terms and conditions of the product that will govern the behavior of the jar.
    pub product_id: String,

    /// Specifies the expiration date of the ticket. The expiration date is measured in milliseconds
    /// since the Unix epoch. This property ensures that the request to create a jar is valid only
    /// until the specified timestamp. After this timestamp, the ticket becomes
    /// invalid and should not be accepted.
    pub valid_until: U64,

    /// An optional user timezone. Required for creating step jars.
    pub timezone: Option<Timezone>,
}

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
        amount: U128,
        signature: &Option<Base64VecU8>,
    ) {
        let amount = amount.0;
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

        let account = self.get_or_create_account_mut(&account_id);
        account.deposit(product_id, amount, None);
    }
}
