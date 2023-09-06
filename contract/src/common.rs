use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{self, Deserialize, Deserializer, Serialize, Serializer},
};

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

    pub(crate) fn to_f32(&self) -> f32 {
        self.significand as f32 / 10u128.pow(self.exponent) as f32
    }
}

impl UDecimal {
    pub(crate) fn new(significand: u128, exponent: u32) -> Self {
        Self { significand, exponent }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize, Hash)]
pub struct U32(pub u32);

impl From<u32> for U32 {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<U32> for u32 {
    fn from(v: U32) -> u32 {
        v.0
    }
}

impl Serialize for U32 {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for U32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(Self(
            str::parse::<u32>(&s).map_err(|err| serde::de::Error::custom(err.to_string()))?,
        ))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId, Balance};

    use crate::{common::UDecimal, Contract};

    pub(crate) struct Context {
        pub contract: Contract,
        ft_contract_id: AccountId,
        owner: AccountId,
        builder: VMContextBuilder,
    }

    impl Context {
        pub(crate) fn new(manager: AccountId) -> Self {
            let owner = AccountId::new_unchecked("owner".to_string());
            let fee_account_id = AccountId::new_unchecked("fee".to_string());
            let ft_contract_id = AccountId::new_unchecked("token".to_string());

            let mut builder = VMContextBuilder::new();
            builder
                .current_account_id(owner.clone())
                .signer_account_id(owner.clone())
                .predecessor_account_id(owner.clone())
                .block_timestamp(0);

            testing_env!(builder.build());

            let contract = Contract::init(ft_contract_id.clone(), fee_account_id, manager);

            Self {
                owner,
                ft_contract_id,
                builder,
                contract,
            }
        }

        pub(crate) fn set_block_timestamp_in_days(&mut self, days: u64) {
            self.set_block_timestamp(Duration::from_secs(days * 24 * 60 * 60));
        }

        pub(crate) fn set_block_timestamp_in_minutes(&mut self, hours: u64) {
            self.set_block_timestamp(Duration::from_secs(hours * 60));
        }

        pub(crate) fn set_block_timestamp_in_ms(&mut self, ms: u64) {
            self.set_block_timestamp(Duration::from_millis(ms));
        }

        pub(crate) fn set_block_timestamp(&mut self, duration: Duration) {
            self.builder.block_timestamp(duration.as_nanos() as u64);
            testing_env!(self.builder.build());
        }

        pub(crate) fn switch_account(&mut self, account_id: &AccountId) {
            self.builder
                .predecessor_account_id(account_id.clone())
                .signer_account_id(account_id.clone());
            testing_env!(self.builder.build());
        }

        pub(crate) fn switch_account_to_owner(&mut self) {
            self.switch_account(&self.owner.clone());
        }

        pub(crate) fn switch_account_to_ft_contract_account(&mut self) {
            self.switch_account(&self.ft_contract_id.clone());
        }

        pub(crate) fn with_deposit_yocto(&mut self, amount: Balance, f: impl FnOnce(&mut Context) -> ()) {
            self.set_deposit_yocto(amount);

            f(self);

            self.set_deposit_yocto(0);
        }

        fn set_deposit_yocto(&mut self, amount: Balance) {
            self.builder.attached_deposit(amount);
            testing_env!(self.builder.build());
        }
    }

    #[test]
    fn udecimal_to_f32() {
        let udecimal = UDecimal::new(12, 2);
        let float_value = udecimal.to_f32();

        assert_eq!(0.12, float_value);
    }
}
