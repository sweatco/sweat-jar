use std::collections::HashMap;

use near_sdk::{env, env::panic_str, near};

use crate::{
    data::{
        jar::{Deposit, Jar, JarCompanion},
        product::{Product, ProductId},
        score::AccountScore,
    },
    interest::InterestCalculator,
    Timestamp, Timezone, TokenAmount,
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

impl AccountV1 {
    pub fn get_total_principal(&self) -> TokenAmount {
        self.jars
            .iter()
            .fold(TokenAmount::default(), |acc, (_, jar)| acc + jar.total_principal())
    }

    pub fn get_jar(&self, product_id: &ProductId) -> &Jar {
        self.jars
            .get(product_id)
            .unwrap_or_else(|| panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub fn get_jar_mut(&mut self, product_id: &ProductId) -> &mut Jar {
        self.jars
            .get_mut(product_id)
            .unwrap_or_else(|| panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub fn deposit(&mut self, product_id: &ProductId, principal: TokenAmount, time: Option<Timestamp>) {
        let deposit = Deposit::new(time.unwrap_or_else(env::block_timestamp_ms), principal);
        let jar = self.jars.entry(product_id.clone()).or_default();
        jar.deposits.push(deposit);
    }

    pub fn try_set_timezone(&mut self, timezone: Option<Timezone>) {
        match (timezone, self.score.is_valid()) {
            // Time zone already set. No actions required.
            (Some(_) | None, true) => (),
            (Some(timezone), false) => {
                self.score = AccountScore::new(timezone);
            }
            (None, false) => {
                panic_str("Trying to create score based jar without providing time zone");
            }
        }
    }

    pub fn apply(&mut self, companion: &AccountV1Companion) {
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

    pub fn update_jar_cache(&mut self, product: &Product, now: Timestamp) {
        let jar = self.get_jar(&product.id);
        let (interest, remainder) = product.terms.get_interest(self, jar, now);
        self.get_jar_mut(&product.id).update_cache(interest, remainder, now);
    }
}
