use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    env, near_bindgen, require,
    serde::{Deserialize, Serialize},
    store::{LookupMap, UnorderedMap},
    AccountId, PanicOnDefault,
};
use sweat_jar_model::{api::MigrationToJarWithRoundingErrorApi, jar::JarId, ProductId, TokenAmount};

use crate::{
    common::Timestamp, jar::model::JarCache, product::model::Product, AccountJars, Contract, ContractExt, Jar,
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
    pub account_jars: LookupMap<AccountId, AccountJarsBeforeRoundingError>,
}

#[near_bindgen]
impl MigrationToJarWithRoundingErrorApi for Contract {
    #[init(ignore_state)]
    fn migrate_to_jars_with_rounding_error(users: Vec<AccountId>) -> Self {
        let old_state: ContractBeforeRoundingError = env::state_read().expect("failed");
        let mut new_state: Contract = env::state_read().expect("failed");

        require!(
            old_state.manager == env::predecessor_account_id(),
            "Can be performed only by admin"
        );

        for user in users {
            let jars = old_state
                .account_jars
                .get(&user)
                .unwrap_or_else(|| panic!("User: {user} doesn't exist"));
            new_state.account_jars.insert(user, jars.clone().into());
        }

        new_state
    }
}
