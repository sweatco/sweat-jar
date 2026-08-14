use near_sdk::env::{self, panic_str};

use super::{
    features::{Feature, Features},
    Account, AccountCompanion,
};
use crate::{
    data::{
        jar::{Deposit, Jar},
        product::{Product, ProductId},
        score::DailyScore,
    },
    interest::InterestCalculator,
    Timestamp, Timezone, TokenAmount,
};

pub trait FeaturesAccess {
    fn features(&self) -> &Features;

    fn features_mut(&mut self) -> &mut Features;

    fn is_feature_enabled(&self, feature: &Feature) -> bool {
        self.features().is_feature_enabled(feature)
    }

    fn set_feature_enabled(&mut self, feature: &Feature, enabled: bool) {
        self.features_mut().set_feature_enabled(feature, enabled);
    }
}

impl Account {
    /// True iff this account has never received a deposit, score, feature
    /// flag, or timezone. Used to guard the one-shot account migration
    /// against overwriting an account that already has real state.
    ///
    /// Can't compare against `Self::default()` directly: `AccountScore`'s
    /// `Default` impl stamps `updated_at` with the current block timestamp,
    /// so a struct-equality check would spuriously return `false` for a
    /// genuinely empty account once time has moved on since it was created.
    pub fn is_empty(&self) -> bool {
        self.nonce == 0
            && self.jars.is_empty()
            && !self.timezone.is_valid()
            && self.features == Features::default()
            && self.score.history.iter().all(|daily| *daily == DailyScore::default())
    }

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
        if self.timezone.is_valid() {
            return;
        }

        if let Some(timezone) = timezone {
            self.timezone = timezone;
        } else {
            panic_str("Trying to create score based jar without providing time zone");
        }
    }

    pub fn apply(&mut self, companion: &AccountCompanion) {
        if let Some(nonce) = companion.nonce {
            self.nonce = nonce;
        }

        if let Some(jars) = &companion.jars {
            for (product_id, jar_companion) in jars {
                let jar = self.jars.get_mut(product_id).expect("Jar is not found");
                jar.apply(jar_companion);
            }
        }

        if let Some(timezone) = companion.timezone {
            self.timezone = timezone;
        }

        if let Some(score) = companion.score {
            self.score = score;
        }

        if let Some(features) = companion.features {
            self.features = features;
        }
    }

    pub fn update_jar_cache(&mut self, product: &Product, now: Timestamp) {
        let jar = self.get_jar(&product.id);
        let (interest, remainder) = product.terms.get_interest(self, jar, now);
        self.get_jar_mut(&product.id).update_cache(interest, remainder, now);
    }

    pub fn is_timezone_set(&self) -> bool {
        self.timezone.is_valid()
    }

    pub fn assert_no_pending_score(&self) {
        assert_eq!(
            self.score.get_days_number_since_last_update(self.timezone),
            0,
            "Account has pending score"
        );
    }
}

impl FeaturesAccess for Account {
    fn features(&self) -> &Features {
        &self.features
    }

    fn features_mut(&mut self) -> &mut Features {
        &mut self.features
    }
}
