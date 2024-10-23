use std::collections::HashMap;

use near_sdk::{env, env::panic_str, near, AccountId};
use sweat_jar_model::{ProductId, Timezone, TokenAmount};

use crate::{
    common::Timestamp,
    jar::model::{Deposit, JarCache, JarV2, JarV2Companion},
    product::model::v2::{InterestCalculator, ProductV2},
    score::AccountScore,
    Contract,
};

#[near]
#[derive(Default, Debug, PartialEq)]
pub struct AccountV2 {
    /// Is used as nonce in `get_ticket_hash` method.
    /// TODO: check logic behind it
    /// TODO: doc change for BE migration
    pub nonce: u32,
    pub jars: HashMap<ProductId, JarV2>,
    pub score: AccountScore,
    pub is_penalty_applied: bool,
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq)]
pub struct AccountV2Companion {
    pub nonce: Option<u32>,
    pub jars: Option<HashMap<ProductId, JarV2Companion>>,
    pub score: Option<AccountScore>,
    pub is_penalty_applied: Option<bool>,
}

impl Contract {
    pub(crate) fn try_get_account(&self, account_id: &AccountId) -> Option<&AccountV2> {
        self.accounts_v2.get(account_id)
    }

    pub(crate) fn get_account(&self, account_id: &AccountId) -> &AccountV2 {
        self.accounts_v2
            .get(account_id)
            .unwrap_or_else(|| env::panic_str(format!("Account {account_id} is not found").as_str()))
    }

    pub(crate) fn get_account_mut(&mut self, account_id: &AccountId) -> &mut AccountV2 {
        self.accounts_v2
            .get_mut(account_id)
            .unwrap_or_else(|| env::panic_str(format!("Account {account_id} is not found").as_str()))
    }

    pub(crate) fn get_or_create_account_mut(&mut self, account_id: &AccountId) -> &mut AccountV2 {
        if !self.accounts_v2.contains_key(account_id) {
            self.accounts_v2.insert(account_id.clone(), AccountV2::default());
        }

        self.accounts_v2.get_mut(account_id).expect("Account is not presented")
    }
}

impl AccountV2 {
    pub(crate) fn get_jar(&self, product_id: &ProductId) -> &JarV2 {
        self.jars
            .get(product_id)
            .unwrap_or_else(|| env::panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub(crate) fn get_jar_mut(&mut self, product_id: &ProductId) -> &mut JarV2 {
        self.jars
            .get_mut(product_id)
            .unwrap_or_else(|| env::panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub(crate) fn deposit(&mut self, product_id: &ProductId, principal: TokenAmount) {
        let deposit = Deposit::new(env::block_timestamp_ms(), principal);
        self.push(product_id, deposit);
    }

    // TODO: refactor, move to some container
    pub(crate) fn push(&mut self, product_id: &ProductId, deposit: Deposit) {
        if let Some(jar) = self.jars.get_mut(product_id) {
            jar.deposits.push(deposit);
        } else {
            let mut jar = JarV2::default();
            jar.deposits.push(deposit);

            self.jars.insert(product_id.clone(), jar);
        }
    }

    pub(crate) fn try_set_timezone(&mut self, timezone: Option<Timezone>) {
        match (timezone, self.score.is_valid()) {
            // Time zone already set. No actions required.
            (Some(_) | None, true) => (),
            (Some(timezone), false) => {
                self.score = AccountScore::new(timezone);
            }
            (None, false) => {
                panic_str("Trying to create score based jar for without providing time zone");
            }
        }
    }

    pub(crate) fn apply(&mut self, companion: &AccountV2Companion) {
        if let Some(nonce) = companion.nonce {
            self.nonce = nonce;
        }

        if let Some(jars) = &companion.jars {
            for (product_id, jar_companion) in jars {
                let jar = self.jars.get_mut(product_id).expect("Jar is not found");
                jar.apply(jar_companion);
            }
        }

        if let Some(score) = companion.score {
            self.score = score;
        }

        if let Some(is_penalty_applied) = companion.is_penalty_applied {
            self.is_penalty_applied = is_penalty_applied;
        }
    }

    fn update_jar_cache(&mut self, product: &ProductV2, now: Timestamp) {
        let jar = self.get_jar(&product.id);
        let (interest, remainder) = product.terms.get_interest(self, jar, now);
        self.get_jar_mut(&product.id).update_cache(interest, remainder, now);
    }
}

impl Contract {
    pub(crate) fn update_account_cache(
        &mut self,
        account_id: &AccountId,
        filter: Option<Box<dyn FnMut(&ProductV2) -> bool>>,
    ) {
        let now = env::block_timestamp_ms();
        let products = self.get_products(account_id, filter);
        let account = self.get_account_mut(account_id);

        for product in products {
            account.update_jar_cache(&product, now);
        }
    }

    pub(crate) fn update_jar_cache(&mut self, account_id: &AccountId, product_id: &ProductId) {
        let product = &self.get_product(product_id);
        let account = self.get_account_mut(account_id);
        account.update_jar_cache(product, env::block_timestamp_ms());
    }

    fn get_products<P>(&self, account_id: &AccountId, filter: Option<P>) -> Vec<ProductV2>
    where
        P: FnMut(&ProductV2) -> bool,
    {
        let products = self
            .get_account(&account_id)
            .jars
            .iter()
            .map(|(product_id, _)| self.get_product(product_id));

        if let Some(filter) = filter {
            products.filter(filter).collect()
        } else {
            products.collect()
        }
    }
}

impl JarV2 {
    fn update_cache(&mut self, interest: TokenAmount, remainder: u64, now: Timestamp) {
        self.cache = Some(JarCache {
            updated_at: now,
            interest,
        });
        // TODO: adjust remainder
        self.claim_remainder += remainder;
    }
}
