use near_sdk::near;

#[near(serializers=[borsh, json])]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueTier<T> {
    pub default: T,
    pub fallback: T,
}

#[near(serializers=[borsh, json])]
#[serde(
    into = "serde_helpers::ConfigurableValueDto<T>",
    from = "serde_helpers::ConfigurableValueDto<T>"
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConfigurableValue<T: Clone> {
    Constant(T),
    Tier(ValueTier<T>),
}

impl<T: Clone> ConfigurableValue<T> {
    /// Every concrete value this `ConfigurableValue` can resolve to —
    /// `[value]` for `Constant`, `[default, fallback]` for `Tier`.
    pub fn values(&self) -> Vec<&T> {
        match self {
            ConfigurableValue::Constant(value) => vec![value],
            ConfigurableValue::Tier(tier) => vec![&tier.default, &tier.fallback],
        }
    }
}

pub mod serde_helpers {
    use near_sdk::near;

    use super::{ConfigurableValue, ValueTier};

    #[near(serializers=[json])]
    pub struct ConfigurableValueDto<T> {
        default: T,
        #[serde(skip_serializing_if = "Option::is_none")]
        fallback: Option<T>,
    }

    impl<T: Clone> From<ConfigurableValue<T>> for ConfigurableValueDto<T> {
        fn from(value: ConfigurableValue<T>) -> Self {
            match value {
                ConfigurableValue::Constant(value) => Self {
                    default: value,
                    fallback: None,
                },
                ConfigurableValue::Tier(value) => Self {
                    default: value.default,
                    fallback: Some(value.fallback),
                },
            }
        }
    }

    impl<T: Clone> From<ConfigurableValueDto<T>> for ConfigurableValue<T> {
        fn from(dto: ConfigurableValueDto<T>) -> Self {
            match dto.fallback {
                Some(fallback) => ConfigurableValue::Tier(ValueTier {
                    default: dto.default,
                    fallback,
                }),
                None => ConfigurableValue::Constant(dto.default),
            }
        }
    }
}
