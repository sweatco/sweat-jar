use near_sdk::near;
use sweat_jar_model::{ProductId, Score};

use crate::{
    common::Duration,
    product::model::common::{Apy, Cap, WithdrawalFee},
};

/// The `Product` struct describes the terms of a deposit jar. It can be of Flexible or Fixed type.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub(crate) struct ProductLegacy {
    /// The unique identifier of the product.
    pub id: ProductId,

    /// The Annual Percentage Yield (APY) associated with the product.
    pub apy: Apy,

    /// The capacity boundaries of the deposit jar, specifying the minimum and maximum principal amount.
    pub cap: Cap,

    /// The terms specific to the product, which can be either Flexible or Fixed.
    pub terms: Terms,

    /// Describes whether a withdrawal fee is applicable and, if so, its details.
    pub withdrawal_fee: Option<WithdrawalFee>,

    /// An optional ed25519 public key used for authorization to create a jar for this product.
    pub public_key: Option<Vec<u8>>,

    /// Indicates whether it's possible to create a new jar for this product.
    pub is_enabled: bool,

    /// TODO: document 0 - non step jar
    pub score_cap: Score,
}

/// The `Terms` enum describes additional terms specific to either Flexible or Fixed products.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub(crate) enum Terms {
    /// Describes additional terms for Fixed products.
    Fixed(FixedProductTerms),

    /// Describes additional terms for Flexible products.
    Flexible,
}

/// The `FixedProductTerms` struct contains terms specific to Fixed products.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FixedProductTerms {
    /// The maturity term of the jar, during which it yields interest. After this period, the user can withdraw principal
    /// or potentially restake the jar.
    pub lockup_term: Duration,

    /// Indicates whether a user can refill the jar.
    pub allows_top_up: bool,

    /// Indicates whether a user can restake the jar after maturity.
    pub allows_restaking: bool,
}
