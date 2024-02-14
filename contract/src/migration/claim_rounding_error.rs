use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    serde::{Deserialize, Serialize},
    store::{LookupMap, UnorderedMap},
    AccountId, PanicOnDefault,
};
use sweat_jar_model::{api::MigrationToClaimRemainder, jar::JarId, ProductId, TokenAmount};

use crate::{
    common::Timestamp, jar::model::JarCache, product::model::Product, AccountJars, Contract, ContractExt, Jar,
    StorageKey,
};

#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
struct JarBeforeClaimRemainder {
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

impl From<JarBeforeClaimRemainder> for Jar {
    #[mutants::skip]
    fn from(value: JarBeforeClaimRemainder) -> Self {
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

#[derive(Default, Debug, Clone, BorshDeserialize, BorshSerialize)]
struct AccountJarsBeforeClaimRemainder {
    last_id: JarId,
    jars: Vec<JarBeforeClaimRemainder>,
}

impl From<AccountJarsBeforeClaimRemainder> for AccountJars {
    #[mutants::skip]
    fn from(value: AccountJarsBeforeClaimRemainder) -> Self {
        AccountJars {
            last_id: value.last_id,
            jars: value.jars.into_iter().map(Into::into).collect(),
        }
    }
}

type AccountJarsBeforeRemainder = LookupMap<AccountId, AccountJarsBeforeClaimRemainder>;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
struct ContractBeforeClaimRemainder {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub last_jar_id: JarId,
    pub account_jars: AccountJarsBeforeRemainder,
}

#[near_bindgen]
impl MigrationToClaimRemainder for Contract {
    #[init(ignore_state)]
    #[mutants::skip]
    fn migrate_state_to_claim_remainder() -> Self {
        let old_state: ContractBeforeClaimRemainder = env::state_read().expect("failed");

        Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products: old_state.products,
            last_jar_id: old_state.last_jar_id,
            account_jars: LookupMap::new(StorageKey::AccountJarsV2),
        }
    }

    #[mutants::skip]
    fn migrate_accounts_to_claim_remainder(&mut self, accounts: Vec<AccountId>) {
        let mut old_account_jars: AccountJarsBeforeRemainder = LookupMap::new(StorageKey::AccountJars);

        for account in accounts {
            let jars = old_account_jars
                .remove(&account)
                .unwrap_or_else(|| panic!("User: {account} does not exist"));

            self.account_jars.insert(account, jars.into());
        }
    }
}
