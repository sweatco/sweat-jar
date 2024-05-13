use near_sdk::{near_bindgen, serde_json, serde_json::json, AccountId, Gas, NearToken, Promise};
use sweat_jar_model::{withdraw::Fee, TokenAmount};

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
    fn ft_transfer(&self, receiver_id: &AccountId, amount: u128, memo: &str, fee: &Option<Fee>) -> Promise;
}

impl FungibleTokenInterface for FungibleTokenContract {
    #[mutants::skip] // Covered by integration tests
    fn ft_transfer(&self, receiver_id: &AccountId, amount: u128, memo: &str, fee: &Option<Fee>) -> Promise {
        if let Some(fee) = fee {
            Promise::new(self.address.clone())
                .ft_transfer(receiver_id, amount - fee.amount, Some(memo.to_string()))
                .ft_transfer(&fee.beneficiary_id, fee.amount, Some(format!("{memo} fee")))
        } else {
            Promise::new(self.address.clone()).ft_transfer(receiver_id, amount, Some(memo.to_string()))
        }
    }
}

trait FungibleTokenPromise {
    fn ft_transfer(self, receiver_id: &AccountId, amount: TokenAmount, memo: Option<String>) -> Promise;
}

impl FungibleTokenPromise for Promise {
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
