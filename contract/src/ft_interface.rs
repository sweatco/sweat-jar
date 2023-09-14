use near_sdk::{
    near_bindgen,
    serde::{Deserialize, Serialize},
    serde_json::json,
    AccountId, Promise,
};

use crate::{common::TokenAmount, Contract, ContractExt, Gas};

pub(crate) const fn tgas(val: u64) -> Gas {
    Gas(Gas::ONE_TERA.0 * val)
}

pub(crate) const GAS_FOR_AFTER_TRANSFER: Gas = tgas(20);

pub(crate) struct FungibleTokenContract {
    address: AccountId,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Fee {
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
    fn transfer(&self, receiver_id: &AccountId, amount: u128, memo: &str, fee: &Option<Fee>) -> Promise;
}

impl FungibleTokenInterface for FungibleTokenContract {
    fn transfer(&self, receiver_id: &AccountId, amount: u128, memo: &str, fee: &Option<Fee>) -> Promise {
        if let Some(fee) = fee {
            Promise::new(self.address.clone())
                .ft_transfer(receiver_id, amount - fee.amount, Some(memo.to_string()))
                .ft_transfer(&fee.beneficiary_id, fee.amount, Some(format!("{memo} fee")))
        } else {
            Promise::new(self.address.clone()).ft_transfer(receiver_id, amount, Some(memo.to_string()))
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
            "memo": memo.unwrap_or_default(),
        })
        .to_string()
        .as_bytes()
        .to_vec();

        self.function_call("ft_transfer".to_string(), args, 1, tgas(5))
    }
}
