use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
    serde_json, PromiseOrValue,
};

use crate::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FtMessage {
    Stake(),
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        println!("Got {:?} tokens from {:?}", amount, sender_id);

        let ft_message: FtMessage = serde_json::from_str(&msg).unwrap();

        match ft_message {
            FtMessage::Stake() => {}
        }

        PromiseOrValue::Value(0.into())
    }
}
