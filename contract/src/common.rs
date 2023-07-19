pub(crate) const MINUTES_IN_YEAR: Duration = 365 * 24 * 60;
pub(crate) const MS_IN_MINUTE: u64 = 1000 * 60;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

/// Amount of fungible tokens
pub type TokenAmount = u128;

pub mod u128_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
        where
            D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use near_sdk::{AccountId, testing_env};
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
    }

    fn days_to_nano_ms(days: u64) -> u64 {
        minutes_to_nano_ms(days * 60 * 24)
    }

    fn minutes_to_nano_ms(minutes: u64) -> u64 {
        minutes * 60 * u64::pow(10, 9)
    }
}