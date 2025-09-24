use near_sdk::near;

#[near(serializers = [borsh, json])]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct Features(u8);

impl Features {
    const INCREASED_APY_BIT: u8 = 0;
    const INCREASED_SCORE_CAP_BIT: u8 = 1;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_increased_apy_enabled(&self) -> bool {
        self.0 & (1 << Self::INCREASED_APY_BIT) != 0
    }

    pub fn is_increased_score_cap_enabled(&self) -> bool {
        self.0 & (1 << Self::INCREASED_SCORE_CAP_BIT) != 0
    }

    pub fn set_increased_apy_enabled(&mut self, enabled: bool) {
        self.set_feature_enabled(Self::INCREASED_APY_BIT, enabled);
    }

    pub fn set_increased_score_cap_enabled(&mut self, enabled: bool) {
        self.set_feature_enabled(Self::INCREASED_SCORE_CAP_BIT, enabled);
    }
    fn set_feature_enabled(&mut self, bit_position: u8, enabled: bool) {
        if enabled {
            self.0 |= 1 << bit_position;
        } else {
            self.0 &= !(1 << bit_position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Features {
        /// Create a new Features instance with specific features enabled
        pub fn with_features(increased_apy: bool, increased_score_cap: bool) -> Self {
            let mut features = Self::new();
            features.set_increased_apy_enabled(increased_apy);
            features.set_increased_score_cap_enabled(increased_score_cap);
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
        assert!(!features.is_increased_apy_enabled());
        assert!(!features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_with_features_constructor() {
        let features = Features::with_features(true, false);
        assert!(features.is_increased_apy_enabled());
        assert!(!features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000001);

        let features = Features::with_features(false, true);
        assert!(!features.is_increased_apy_enabled());
        assert!(features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000010);

        let features = Features::with_features(true, true);
        assert!(features.is_increased_apy_enabled());
        assert!(features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000011);
    }

    #[test]
    fn test_set_increased_apy_enabled() {
        let mut features = Features::new();

        features.set_increased_apy_enabled(true);
        assert!(features.is_increased_apy_enabled());
        assert!(!features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000001);

        features.set_increased_apy_enabled(false);
        assert!(!features.is_increased_apy_enabled());
        assert!(!features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_set_increased_score_cap_enabled() {
        let mut features = Features::new();

        features.set_increased_score_cap_enabled(true);
        assert!(!features.is_increased_apy_enabled());
        assert!(features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000010);

        features.set_increased_score_cap_enabled(false);
        assert!(!features.is_increased_apy_enabled());
        assert!(!features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_both_features_together() {
        let mut features = Features::new();

        // Enable both features
        features.set_increased_apy_enabled(true);
        features.set_increased_score_cap_enabled(true);
        assert!(features.is_increased_apy_enabled());
        assert!(features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000011);

        // Disable one feature
        features.set_increased_apy_enabled(false);
        assert!(!features.is_increased_apy_enabled());
        assert!(features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000010);

        // Disable the other feature
        features.set_increased_score_cap_enabled(false);
        assert!(!features.is_increased_apy_enabled());
        assert!(!features.is_increased_score_cap_enabled());
        assert_eq!(features.raw_value(), 0x00000000);
    }

    #[test]
    fn test_clone_and_equality() {
        let features1 = Features::with_features(true, false);
        let features2 = features1.clone();

        assert_eq!(features1, features2);
        assert!(features1.is_increased_apy_enabled());
        assert!(!features1.is_increased_score_cap_enabled());
    }
}
