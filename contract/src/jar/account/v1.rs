use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use near_sdk::{env, env::panic_str, near, AccountId};
use sweat_jar_model::{ProductId, Timezone, TokenAmount};

use crate::{
    common::Timestamp,
    jar::{
        account::Account,
        model::{Deposit, Jar, JarCache, JarCompanion},
    },
    product::model::v1::{InterestCalculator, Product},
    score::AccountScore,
    Contract,
};

#[near]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountV1 {
    /// TODO: doc change for BE migration
    pub nonce: u32,
    pub jars: HashMap<ProductId, Jar>,
    pub score: AccountScore,
    pub is_penalty_applied: bool,
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq)]
pub struct AccountV1Companion {
    pub nonce: Option<u32>,
    pub jars: Option<HashMap<ProductId, JarCompanion>>,
    pub score: Option<AccountScore>,
    pub is_penalty_applied: Option<bool>,
}

impl Contract {
    pub(crate) fn try_get_account(&self, account_id: &AccountId) -> Option<&Account> {
        self.accounts.get(account_id).map(Deref::deref)
    }

    pub(crate) fn get_account(&self, account_id: &AccountId) -> &Account {
        self.try_get_account(account_id)
            .unwrap_or_else(|| panic_str(format!("Account {account_id} is not found").as_str()))
    }

    pub(crate) fn get_account_mut(&mut self, account_id: &AccountId) -> &mut Account {
        self.accounts
            .get_mut(account_id)
            .unwrap_or_else(|| panic_str(format!("Account {account_id} is not found").as_str()))
            .deref_mut()
    }

    pub(crate) fn get_or_create_account_mut(&mut self, account_id: &AccountId) -> &mut Account {
        self.accounts.entry(account_id.clone()).or_default()
    }
}

impl AccountV1 {
    pub(crate) fn get_jar(&self, product_id: &ProductId) -> &Jar {
        self.jars
            .get(product_id)
            .unwrap_or_else(|| panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub(crate) fn get_jar_mut(&mut self, product_id: &ProductId) -> &mut Jar {
        self.jars
            .get_mut(product_id)
            .unwrap_or_else(|| panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub(crate) fn deposit(&mut self, product_id: &ProductId, principal: TokenAmount, time: Option<Timestamp>) {
        let deposit = Deposit::new(time.unwrap_or_else(env::block_timestamp_ms), principal);
        let jar = self.jars.entry(product_id.clone()).or_default();
        jar.deposits.push(deposit);
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

    pub(crate) fn apply(&mut self, companion: &AccountV1Companion) {
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

    pub(crate) fn update_jar_cache(&mut self, product: &Product, now: Timestamp) {
        let jar = self.get_jar(&product.id);
        let (interest, remainder) = product.terms.get_interest(self, jar, now);
        self.get_jar_mut(&product.id).update_cache(interest, remainder, now);
    }
}

impl Contract {
    pub(crate) fn update_account_cache(&mut self, account_id: &AccountId, filter: Option<fn(&Product) -> bool>) {
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

    fn get_products(&self, account_id: &AccountId, filter: Option<impl Fn(&Product) -> bool>) -> Vec<Product> {
        let products = self
            .get_account(account_id)
            .jars
            .keys()
            .map(|product_id| self.get_product(product_id));

        if let Some(filter) = filter {
            products.filter(filter).collect()
        } else {
            products.collect()
        }
    }
}

impl Jar {
    pub(crate) fn update_cache(&mut self, interest: TokenAmount, remainder: u64, now: Timestamp) {
        self.cache = Some(JarCache {
            updated_at: now,
            interest,
        });
        self.claim_remainder = remainder;
    }
}
