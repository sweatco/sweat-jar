use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{json_types::U128, AccountId, Promise, near_bindgen};
use crate::*;
use crate::common::TokenAmount;

pub(crate) struct FungibleTokenContract {
    address: AccountId,
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
    fn transfer(&self, receiver_id: AccountId, amount: u128, callback: Promise) -> PromiseOrValue<TokenAmount>;
}

impl FungibleTokenInterface for FungibleTokenContract {
    fn transfer(&self, receiver_id: AccountId, amount: u128, callback: Promise) -> PromiseOrValue<TokenAmount> {
        ext_ft_core::ext(self.address.clone())
            .with_attached_deposit(1)
            .ft_transfer(receiver_id, U128::from(amount), None)
            .then(callback)
            .into()
    }
}
