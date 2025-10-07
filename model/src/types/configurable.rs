use near_sdk::near;

#[near(serializers=[borsh, json])]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueTier<T> {
    pub default: T,
    pub fallback: T,
}

#[near(serializers=[borsh, json])]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConfigurableValue<T> {
    Constant(T),
    Tier(ValueTier<T>),
}
