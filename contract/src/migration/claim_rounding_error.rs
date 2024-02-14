use std::mem::{forget, size_of, transmute};

use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    env::used_gas,
    log, near_bindgen,
    serde::{Deserialize, Serialize},
    store::{LookupMap, UnorderedMap},
    AccountId, Gas, IntoStorageKey, PanicOnDefault,
};
use sweat_jar_model::{api::MigrationToJarWithRoundingErrorApi, jar::JarId, ProductId, TokenAmount};

use crate::{
    common::Timestamp, jar::model::JarCache, product::model::Product, AccountJars, Contract, ContractExt, Jar,
    StorageKey,
};

#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
pub struct JarBeforeRoundingError {
    pub id: JarId,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: Timestamp,
    pub principal: TokenAmount,
    pub cache: Option<JarCache>,
    pub claimed_balance: TokenAmount,
    pub is_pending_withdraw: bool,
    pub is_penalty_applied: bool,
}

impl From<JarBeforeRoundingError> for Jar {
    fn from(value: JarBeforeRoundingError) -> Self {
        Jar {
            id: value.id,
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: value.created_at,
            principal: value.principal,
            cache: value.cache,
            claimed_balance: value.claimed_balance,
            is_pending_withdraw: value.is_pending_withdraw,
            is_penalty_applied: value.is_penalty_applied,
            claim_remainder: 0,
        }
    }
}

#[derive(Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct AccountJarsBeforeRoundingError {
    pub last_id: JarId,
    pub jars: Vec<JarBeforeRoundingError>,
}

impl From<AccountJarsBeforeRoundingError> for AccountJars {
    fn from(value: AccountJarsBeforeRoundingError) -> Self {
        AccountJars {
            last_id: value.last_id,
            jars: value.jars.into_iter().map(Into::into).collect(),
        }
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ContractBeforeRoundingError {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub last_jar_id: JarId,
    pub account_jars: AccountJarsMap,
}

type AccountJarsMap = LookupMap<AccountId, AccountJarsBeforeRoundingError>;

const ACCOUNT_JARS_MAP_SIZE: usize = size_of::<AccountJarsMap>();
const ACCOUNT_JARS_PREFIX_SIZE: usize = size_of::<Box<[u8]>>();
const ACCOUNT_JARS_CACHE_SIZE: usize = ACCOUNT_JARS_MAP_SIZE - ACCOUNT_JARS_PREFIX_SIZE;

struct LookupMapPrefix {
    prefix: Box<[u8]>,
    _cache: [u8; ACCOUNT_JARS_CACHE_SIZE],
}

#[near_bindgen]
impl MigrationToJarWithRoundingErrorApi for Contract {
    #[init(ignore_state)]
    fn migrate_to_jars_with_rounding_error(users: Vec<AccountId>) -> Self {
        log!("begin: {:?}", used_gas().0 / Gas::ONE_TERA.0);

        let mut old_state: ContractBeforeRoundingError = env::state_read().expect("failed");

        log!("parsed old state: {:?}", used_gas().0 / Gas::ONE_TERA.0);

        let mut new_state = Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products: old_state.products,
            last_jar_id: old_state.last_jar_id,
            // account_jars: LookupMap::new(StorageKey::AccountJars),
            account_jars: LookupMap::new(b"-"),
            total_jars_count: 0,
        };

        log!("Users: {:?} - {:?}", users, used_gas().0 / Gas::ONE_TERA.0);

        for user in users {
            let jars = old_state
                .account_jars
                .get(&user)
                .unwrap_or_else(|| panic!("User: {user} doesn't exist"))
                .clone();

            log!("got jars - {:?}", used_gas().0 / Gas::ONE_TERA.0);

            let jars = old_state
                .account_jars
                .get(&user)
                .unwrap_or_else(|| panic!("User: {user} doesn't exist"))
                .clone();

            log!("got jars again - {:?}", used_gas().0 / Gas::ONE_TERA.0);

            old_state.account_jars.flush();

            log!("flushed old - {:?}", used_gas().0 / Gas::ONE_TERA.0);

            new_state.account_jars.insert(user, jars.clone().into());

            log!("inserted jars- {:?}", used_gas().0 / Gas::ONE_TERA.0);
        }

        let editable_prefix: &mut LookupMapPrefix = unsafe { transmute(&mut new_state.account_jars) };
        editable_prefix.prefix = StorageKey::AccountJars.into_storage_key().into_boxed_slice();
        forget(old_state.account_jars);

        log!("forget - {:?}", used_gas().0 / Gas::ONE_TERA.0);

        new_state.account_jars.flush();

        log!("flush - {:?}", used_gas().0 / Gas::ONE_TERA.0);

        new_state
    }
}
