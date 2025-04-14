use near_sdk::{
    env,
    json_types::Base64VecU8,
    store::key::{Identity, ToKey},
    AccountId, IntoStorageKey,
};

use crate::{Contract, StorageKey};

impl Contract {
    pub(crate) fn store_account_raw(&mut self, account_id: AccountId, account_bytes: Base64VecU8) {
        let key = Identity::to_key(
            &StorageKey::Accounts.into_storage_key(),
            account_id.as_bytes(),
            &mut Vec::new(),
        );
        env::storage_write(&key, &account_bytes.0);
    }
}
