use sweat_jar_model::{
    product::{ApyView, ProductView},
    MS_IN_YEAR,
};

/// Reads the constant APY (as a fraction, e.g. `0.12` for 12%) of a product from the list
/// returned by `get_products`.
pub fn constant_apy(products: &[ProductView], product_id: &str) -> f64 {
    let product = products.iter().find(|p| p.id == product_id).expect("product not found");
    match &product.apy {
        ApyView::Constant(value) => f64::from(*value),
        ApyView::Downgradable(value) => f64::from(value.default),
    }
}

/// Mirrors the contract's linear interest formula (`elapsed_ms * principal * apy / MS_IN_YEAR`,
/// floored). Tests anchor assertions to this — computed from actually measured elapsed
/// on-chain time — instead of a magic number tied to a specific fast-forward duration, so a
/// shorter (faster) fast-forward doesn't require re-deriving hardcoded bounds.
pub fn expected_interest(elapsed_ms: u64, principal: u128, apy: f64) -> u128 {
    (elapsed_ms as f64 * principal as f64 * apy / MS_IN_YEAR as f64).floor() as u128
}
