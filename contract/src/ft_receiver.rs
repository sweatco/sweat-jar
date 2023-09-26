use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    json_types::U128,
    require,
    serde::{Deserialize, Serialize},
    serde_json, PromiseOrValue,
};

use crate::{
    jar::model::JarTicket, migration::model::CeFiJar, near_bindgen, AccountId, Base64VecU8, Contract, ContractExt,
    JarId,
};

/// The `FtMessage` enum represents various commands for actions available via transferring tokens to an account
/// where this contract is deployed, using the payload in `ft_transfer_call`.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum FtMessage {
    /// Represents a request to create a new jar for a corresponding product.
    Stake(StakeMessage),

    /// Represents a request to create DeFi Jars from provided CeFi Jars.
    Migrate(Vec<CeFiJar>),

    /// Represents a request to refill (top up) an existing jar using its `JarId`.
    TopUp(JarId),
}

/// The `StakeMessage` struct represents a request to create a new jar for a corresponding product.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StakeMessage {
    /// Data of the `JarTicket` required for validating the request and specifying the product.
    ticket: JarTicket,

    /// An optional ed25519 signature used to verify the authenticity of the request.
    signature: Option<Base64VecU8>,

    /// An optional account ID representing the intended owner of the created jar.
    receiver_id: Option<AccountId>,
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        self.assert_from_ft_contract();

        let ft_message: FtMessage = serde_json::from_str(&msg).expect("Unable to deserialize msg");

        match ft_message {
            FtMessage::Stake(message) => {
                let receiver_id = message.receiver_id.unwrap_or(sender_id);
                self.create_jar(receiver_id, message.ticket, amount, message.signature);
            }
            FtMessage::Migrate(jars) => {
                require!(sender_id == self.manager, "Migration can be performed only by admin");

                self.migrate_jars(jars, amount);
            }
            FtMessage::TopUp(jar_id) => {
                self.top_up(&sender_id, jar_id, amount);
            }
        }

        PromiseOrValue::Value(0.into())
    }
}
