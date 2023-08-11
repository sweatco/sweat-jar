use near_sdk::{AccountId, near_bindgen, Promise};
use near_sdk::serde_json::json;

use crate::*;
use crate::common::TokenAmount;

pub(crate) struct FungibleTokenContract {
    address: AccountId,
}

pub(crate) struct Fee {
    pub beneficiary_id: AccountId,
    pub amount: TokenAmount,
}

impl FungibleTokenContract {
    fn new(address: AccountId) -> Self {
        Self { address }
    }
}

#[near_bindgen]
impl Contract {
    pub(crate) fn ft_contract(&self) -> impl FungibleTokenInterface {
        FungibleTokenContract::new(self.token_account_id.clone())
    }
}

pub(crate) trait FungibleTokenInterface {
    fn transfer(&self, receiver_id: &AccountId, amount: u128, fee: Option<Fee>) -> Promise;
}

impl FungibleTokenInterface for FungibleTokenContract {
    fn transfer(&self, receiver_id: &AccountId, amount: u128, fee: Option<Fee>) -> Promise {
        if let Some(fee) = fee {
            Promise::new(self.address.clone())
                //TODO: change memo for "claim"
                .ft_transfer(receiver_id, amount - fee.amount, Some("withdraw".to_string()))
                .ft_transfer(&fee.beneficiary_id, fee.amount, Some("withdraw fee".to_string()))
        } else {
            Promise::new(self.address.clone())
                .ft_transfer(receiver_id, amount, Some("withdraw".to_string()))
        }
    }
}

trait FtTransferPromise {
    fn ft_transfer(self, receiver_id: &AccountId, amount: TokenAmount, memo: Option<String>) -> Promise;
}

impl FtTransferPromise for Promise {
    fn ft_transfer(self, receiver_id: &AccountId, amount: TokenAmount, memo: Option<String>) -> Promise {
        let args = json!({
            "receiver_id": receiver_id,
            "amount": amount.to_string(),
            "memo": memo.unwrap_or("".to_string()),
        }).to_string().as_bytes().to_vec();

        self.function_call("ft_transfer".to_string(), args, 1, Gas(5 * Gas::ONE_TERA.0))
    }
}
