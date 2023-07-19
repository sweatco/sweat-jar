use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
    serde_json, PromiseOrValue,
};
use near_sdk::env::log_str;

use crate::*;
use crate::migration::CeFiJar;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "action", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum FtMessage {
    Stake(StakeMessage),
    Migrate(Vec<CeFiJar>),
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StakeMessage {
    product_id: ProductId,
    signature: Option<String>,
    receiver_id: Option<AccountId>,
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let ft_message: FtMessage = serde_json::from_str(&msg).unwrap();

        match ft_message {
            FtMessage::Stake(message) => {
                let receiver_id = message.receiver_id.unwrap_or_else(|| sender_id.clone());
                self.create_jar(
                    receiver_id,
                    message.product_id,
                    amount.0,
                    message.signature,
                );
            }
            FtMessage::Migrate(jars) => {
                self.migrate_jars(jars, amount.0);
            }
        }

        PromiseOrValue::Value(0.into())
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{AccountId, serde_json};
    use crate::ft_receiver::FtMessage::Migrate;
    use crate::migration::CeFiJar;

    #[test]
    fn test() {
        let data = Migrate(vec![
            CeFiJar {
                id: "hello".to_string(),
                account_id: AccountId::new_unchecked("alice".to_string()),
                product_id: "product_1".to_string(),
                principal: 1000000,
                created_at: 0,
                claimed_amount: 0,
                last_claim_at: None,
            }
        ]);

        println!("JSON: {}", serde_json::to_string(&data).unwrap());
    }
}
