use near_sdk::borsh::BorshSerialize;

use crate::account::{v1::AccountV1, Account};

#[derive(BorshSerialize, Debug, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub enum AccountVersioned {
    V1(AccountV1),
}

impl AccountVersioned {
    pub(crate) fn new(account: Account) -> Self {
        AccountVersioned::V1(account)
    }
}
