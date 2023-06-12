use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{json_types::U128, AccountId, Promise};

pub(crate) struct FungibleTokenContract {
    address: AccountId,
}

impl FungibleTokenContract {
    pub(crate) fn new(address: AccountId) -> Self {
        Self { address }
    }
}

pub(crate) trait FungibleTokenInterface {
    fn transfer(&self, receiver_id: AccountId, amount: u128, callback: Promise) -> Promise;
}

impl FungibleTokenInterface for FungibleTokenContract {
    fn transfer(&self, receiver_id: AccountId, amount: u128, callback: Promise) -> Promise {
        ext_ft_core::ext(self.address.clone())
            .with_attached_deposit(1)
            .ft_transfer(receiver_id.clone(), U128::from(amount), None)
            .then(callback)
    }
}
