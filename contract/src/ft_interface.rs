#![cfg(not(test))]

use near_sdk::{near_bindgen, serde_json, serde_json::json, AccountId, Gas, NearToken, Promise};
use sweat_jar_model::TokenAmount;

use crate::{Contract, ContractExt};

pub(crate) struct FungibleTokenContract {
    address: AccountId,
}

impl FungibleTokenContract {
    #[cfg(not(test))]
    fn new(address: AccountId) -> Self {
        Self { address }
    }
}

#[near_bindgen]
impl Contract {
    #[cfg(not(test))]
    pub(crate) fn ft_contract(&self) -> impl FungibleTokenInterface {
        FungibleTokenContract::new(self.token_account_id.clone())
    }
}

pub(crate) trait FungibleTokenInterface {
    fn ft_transfer(&self, receiver_id: &AccountId, amount: u128, memo: &str) -> Promise;
}

impl FungibleTokenInterface for FungibleTokenContract {
    #[mutants::skip] // Covered by integration tests
    fn ft_transfer(&self, receiver_id: &AccountId, amount: u128, memo: &str) -> Promise {
        Promise::new(self.address.clone()).ft_transfer(receiver_id, amount, Some(memo.to_string()))
    }
}

trait FungibleTokenPromise {
    fn ft_transfer(self, receiver_id: &AccountId, amount: TokenAmount, memo: Option<String>) -> Promise;
}

impl FungibleTokenPromise for Promise {
    #[mutants::skip] // Covered by integration tests
    fn ft_transfer(self, receiver_id: &AccountId, amount: TokenAmount, memo: Option<String>) -> Promise {
        let args = serde_json::to_vec(&json!({
            "receiver_id": receiver_id,
            "amount": amount.to_string(),
            "memo": memo.unwrap_or_default(),
        }))
        .expect("Failed to serialize arguments");

        self.function_call(
            "ft_transfer".to_string(),
            args,
            NearToken::from_yoctonear(1),
            Gas::from_tgas(5),
        )
    }
}
