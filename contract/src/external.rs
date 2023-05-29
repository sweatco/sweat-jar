use near_sdk::ext_contract;

use crate::*;

pub(crate) const GAS_FOR_AFTER_CLAIM: u64 = 20_000_000_000_000;

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn after_claim(&mut self, account_id: AccountId, claimed_balance: Balance) -> Balance;
}

impl SelfCallbacks for Contract {
    fn after_claim(&mut self, account_id: AccountId, claimed_balance: Balance) -> Balance {
        let jar_ids = self
            .account_jars
            .get(&account_id)
            .clone()
            .expect("Account doesn't have jars")
            .clone();

        let jar_ids_iter = jar_ids.iter();
        for i in jar_ids_iter {
            let jar = self
                .jars
                .get(*i as _)
                .expect(format!("Jar on index {} doesn't exist", i).as_ref());

            let updated_jar = Jar {
                last_claim_timestamp: jar.last_claim_attempt_timestamp.clone(),
                last_claim_attempt_timestamp: None,
                ..jar.clone()
            };
            self.jars.replace(*i as _, &updated_jar);
        }

        claimed_balance
    }
}
