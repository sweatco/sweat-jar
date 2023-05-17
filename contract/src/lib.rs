use jar::Jar;
// Find all our documentation at https://docs.near.org
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{near_bindgen, AccountId, Balance};

mod external;
mod ft_receiver;
mod jar;

type InterestRate = f32;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    interest_rate: InterestRate,
    term_in_days: u8,
    jars: LookupMap<AccountId, Jar>,
}

#[near_bindgen]
impl Contract {
    pub fn init(interest_rate: f32, term_in_days: u8) -> Self {
        Self {
            interest_rate,
            term_in_days,
            jars: LookupMap::new(b"d"),
        }
    }

    #[private]
    pub fn stake(&mut self, account_id: AccountId, amount: Balance) -> Balance {
        return 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_absent_deposit_when_request_it_then_return_none() {
//        let contract = Contract::init(0.01, 1);
        //        let interest = contract.get_interest_amount(AccountId::new_unchecked("alice".to_string()));

//        assert_eq!(interest, None);
    }
}
