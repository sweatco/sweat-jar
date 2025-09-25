use near_sdk::near;

#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone, Copy)]
pub enum Feature {
    IncreasedApy,
    IncreasedScoreCap,
}

impl Feature {
    fn get_bit(self) -> u8 {
        match self {
            Self::IncreasedApy => 0,
            Self::IncreasedScoreCap => 1,
        }
    }
}

#[near(serializers = [borsh, json])]
#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct Features(u8);

impl Features {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_feature_enabled(&self, feature: &Feature) -> bool {
        self.0 & (1 << feature.get_bit()) != 0
    }

    pub fn set_feature_enabled(&mut self, feature: &Feature, enabled: bool) {
        if enabled {
            self.0 |= 1 << feature.get_bit();
        } else {
            self.0 &= !(1 << feature.get_bit());
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::serde_json;

    use super::*;
    use crate::data::account::{common::FeaturesAccess, v2::AccountV2};

    impl Features {
        /// Create a new Features instance with specific features enabled
        pub fn with_features(increased_apy: bool, increased_score_cap: bool) -> Self {
            let mut features = Self::new();
            features.set_feature_enabled(&Feature::IncreasedApy, increased_apy);
            features.set_feature_enabled(&Feature::IncreasedScoreCap, increased_score_cap);
            features
        }

        /// Get the raw bit mask value
        pub fn raw_value(&self) -> u8 {
            self.0
        }

        /// Create Features from a raw bit mask value
        pub fn from_raw(value: u8) -> Self {
            Self(value)
        }
    }

    #[test]
    fn test_new_features_all_disabled() {
        let features = Features::new();
        assert!(!features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_with_features_constructor() {
        let features = Features::with_features(true, false);
        assert!(features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000001);

        let features = Features::with_features(false, true);
        assert!(!features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000010);

        let features = Features::with_features(true, true);
        assert!(features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000011);
    }

    #[test]
    fn test_set_increased_apy_enabled() {
        let mut features = Features::new();

        features.set_feature_enabled(&Feature::IncreasedApy, true);
        assert!(features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000001);

        features.set_feature_enabled(&Feature::IncreasedApy, false);
        assert!(!features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_set_increased_score_cap_enabled() {
        let mut features = Features::new();

        features.set_feature_enabled(&Feature::IncreasedScoreCap, true);
        assert!(!features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000010);

        features.set_feature_enabled(&Feature::IncreasedScoreCap, false);
        assert!(!features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_both_features_together() {
        let mut features = Features::new();

        // Enable both features
        features.set_feature_enabled(&Feature::IncreasedApy, true);
        features.set_feature_enabled(&Feature::IncreasedScoreCap, true);
        assert!(features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000011);

        // Disable one feature
        features.set_feature_enabled(&Feature::IncreasedApy, false);
        assert!(!features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000010);

        // Disable the other feature
        features.set_feature_enabled(&Feature::IncreasedScoreCap, false);
        assert!(!features.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features.is_feature_enabled(&Feature::IncreasedScoreCap));
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_clone_and_equality() {
        let features1 = Features::with_features(true, false);
        let features2 = features1.clone();

        assert_eq!(features1, features2);
        assert!(features1.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features1.is_feature_enabled(&Feature::IncreasedScoreCap));
    }

    #[test]
    fn test_features_access_trait_on_account() {
        let mut account = AccountV2::default();

        // Test initial state - all features should be disabled
        assert!(!account.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!account.is_feature_enabled(&Feature::IncreasedScoreCap));

        // Test setting increased APY feature
        account.set_feature_enabled(&Feature::IncreasedApy, true);
        assert!(account.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!account.is_feature_enabled(&Feature::IncreasedScoreCap));

        // Test setting increased score cap feature
        account.set_feature_enabled(&Feature::IncreasedScoreCap, true);
        assert!(account.is_feature_enabled(&Feature::IncreasedApy));
        assert!(account.is_feature_enabled(&Feature::IncreasedScoreCap));

        // Test disabling features
        account.set_feature_enabled(&Feature::IncreasedApy, false);
        assert!(!account.is_feature_enabled(&Feature::IncreasedApy));
        assert!(account.is_feature_enabled(&Feature::IncreasedScoreCap));

        account.set_feature_enabled(&Feature::IncreasedScoreCap, false);
        assert!(!account.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!account.is_feature_enabled(&Feature::IncreasedScoreCap));
    }

    #[test]
    fn test_features_access_direct_features_access() {
        let mut account = AccountV2::default();

        // Test direct access to features field
        let features_ref = account.features();
        assert!(!features_ref.is_feature_enabled(&Feature::IncreasedApy));
        assert!(!features_ref.is_feature_enabled(&Feature::IncreasedScoreCap));

        // Test mutable access
        let features_mut = account.features_mut();
        features_mut.set_feature_enabled(&Feature::IncreasedApy, true);
        features_mut.set_feature_enabled(&Feature::IncreasedScoreCap, true);

        // Verify changes through trait methods
        assert!(account.is_feature_enabled(&Feature::IncreasedApy));
        assert!(account.is_feature_enabled(&Feature::IncreasedScoreCap));
    }

    #[test]
    fn test_feature_enum_bits() {
        // Test that Feature enum returns correct bit positions
        assert_eq!(Feature::IncreasedApy.get_bit(), 0);
        assert_eq!(Feature::IncreasedScoreCap.get_bit(), 1);
    }

    #[test]
    fn test_feature_enum_serialization() {
        // Test that Feature enum can be serialized/deserialized
        let feature_apy = Feature::IncreasedApy;
        let feature_score_cap = Feature::IncreasedScoreCap;

        // These should compile and work with serde
        let _json_apy = serde_json::to_string(&feature_apy).unwrap();
        let _json_score_cap = serde_json::to_string(&feature_score_cap).unwrap();
    }
}
