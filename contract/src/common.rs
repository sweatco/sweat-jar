use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

pub(crate) const MINUTES_IN_YEAR: u64 = 365 * 24 * 60;
pub(crate) const MS_IN_MINUTE: u64 = 1000 * 60;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

/// Amount of fungible tokens
pub type TokenAmount = u128;

/// `UDecimal` represents a scientific representation of decimals.
///
/// The decimal number is represented in the form of `significand` divided by (10 raised to the power of `exponent`).
/// The `significand` and `exponent` are both positive integers.
/// The key components of this structure include:
///
/// * `significand`: The parts of the decimal number that holds significant digits, i.e., all digits including and
///                  following the leftmost nonzero digit.
///
/// * `exponent`: The part of the decimal number that represents the power to which 10 must be raised to yield the original number.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct UDecimal {
    pub significand: u128,
    pub exponent: u32,
}

impl UDecimal {
    pub(crate) fn mul(&self, value: u128) -> u128 {
        value * self.significand / 10u128.pow(self.exponent)
    }
}

impl UDecimal {
    pub(crate) fn new(significand: u128, exponent: u32) -> Self {
        Self {
            significand,
            exponent,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use near_sdk::{AccountId, Balance, testing_env};
    use near_sdk::test_utils::VMContextBuilder;

    use crate::Contract;

    pub(crate) struct Context {
        pub contract: Contract,
        owner: AccountId,
        builder: VMContextBuilder,
    }

    impl Context {
        pub(crate) fn new(admins: Vec<AccountId>) -> Self {
            let owner = AccountId::new_unchecked("owner".to_string());
            let fee_account_id = AccountId::new_unchecked("fee".to_string());
            let ft_contract_id = AccountId::new_unchecked("token".to_string());

            let builder = VMContextBuilder::new()
                .current_account_id(owner.clone())
                .signer_account_id(owner.clone())
                .predecessor_account_id(owner.clone())
                .block_timestamp(0)
                .clone();

            testing_env!(builder.build());

            let contract = Contract::init(
                ft_contract_id,
                fee_account_id,
                admins,
            );

            Self {
                owner,
                builder,
                contract,
            }
        }

        pub(crate) fn set_block_timestamp_in_days(&mut self, hours: u64) {
            self.builder = self.builder.clone()
                .block_timestamp(days_to_nano_ms(hours))
                .clone();
            testing_env!(self.builder.build());
        }

        pub(crate) fn set_block_timestamp_in_minutes(&mut self, hours: u64) {
            self.builder = self.builder.clone()
                .block_timestamp(minutes_to_nano_ms(hours))
                .clone();
            testing_env!(self.builder.build());
        }

        pub(crate) fn switch_account(&mut self, account_id: &AccountId) {
            self.builder = self.builder.clone()
                .predecessor_account_id(account_id.clone())
                .signer_account_id(account_id.clone())
                .clone();
            testing_env!(self.builder.build());
        }

        pub(crate) fn switch_account_to_owner(&mut self) {
            self.switch_account(&self.owner.clone());
        }

        pub(crate) fn with_deposit_yocto(&mut self, amount: Balance) {
            self.builder = self.builder.clone().attached_deposit(amount).clone();
            testing_env!(self.builder.build());
        }
    }

    fn days_to_nano_ms(days: u64) -> u64 {
        minutes_to_nano_ms(days * 60 * 24)
    }

    fn minutes_to_nano_ms(minutes: u64) -> u64 {
        minutes * 60 * u64::pow(10, 9)
    }
}