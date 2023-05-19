use jar::{Jar, JarIndex, Product, ProductId, Stake};
use near_sdk::borsh::maybestd::collections::HashSet;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet, Vector};
use near_sdk::{env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault};

mod external;
mod ft_receiver;
mod internal;
mod jar;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub token_account_id: AccountId,
    pub admin_allowlist: UnorderedSet<AccountId>,

    pub products: LookupMap<ProductId, Product>,

    pub jars: Vector<Jar>,
    pub account_jars: LookupMap<AccountId, HashSet<JarIndex>>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Administrators,
    Products,
    Jars,
    AccountJars,
}

#[near_bindgen]
impl Contract {
    pub fn init(token_account_id: AccountId, admin_allowlist: Vec<AccountId>) -> Self {
        let mut admin_allowlist_set = UnorderedSet::new(StorageKey::Administrators);
        admin_allowlist_set.extend(admin_allowlist.clone().into_iter().map(|item| item.into()));

        Self {
            token_account_id,
            admin_allowlist: admin_allowlist_set,
            products: LookupMap::new(StorageKey::Products),
            jars: Vector::new(StorageKey::Jars),
            account_jars: LookupMap::new(StorageKey::AccountJars),
        }
    }

    #[private]
    pub fn create_jar(
        &mut self,
        account_id: AccountId,
        product_id: ProductId,
        amount: Balance,
    ) -> Jar {
        let index = self.jars.len() as JarIndex;
        let now = env::block_timestamp_ms() * 1000;
        let jar = Jar {
            index,
            product_id,
            stakes: vec![Stake {
                account_id: account_id.clone(),
                amount,
                since: now,
            }],
            last_claim_timestamp: None,
        };

        self.save_jar(&account_id, &jar);

        return jar;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_absent_deposit_when_request_it_then_return_none() {}
}
