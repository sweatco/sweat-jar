use near_sdk::{ext_contract, AccountId, Balance};

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn stake(&mut self, account_id: AccountId, amount: Balance) -> Balance;
}
