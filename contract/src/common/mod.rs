pub(crate) mod test_data;
pub(crate) mod tests;
pub(crate) mod udecimal;
use near_sdk::Gas;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

const TERA: u64 = Gas::ONE_TERA.0;

pub const fn tgas(val: u64) -> Gas {
    Gas(TERA * val)
}

pub mod gas_data {
    use near_sdk::Gas;

    use crate::common::{tgas, TERA};

    pub(crate) const GIGA: u64 = TERA / 1000;

    /// Const of after claim call with 1 jar
    const INITIAL_GAS_FOR_AFTER_CLAIM: u64 = 4 * TERA;

    /// Cost of adding 1 additional jar in after claim call. Measured with `measure_after_claim_total_test`
    const ADDITIONAL_AFTER_CLAIM_JAR_COST: u64 = 80 * GIGA;

    /// Values are measured with `measure_after_claim_total_test`
    /// For now number of jars is arbitrary
    pub(crate) const GAS_FOR_AFTER_CLAIM: Gas =
        Gas(INITIAL_GAS_FOR_AFTER_CLAIM + ADDITIONAL_AFTER_CLAIM_JAR_COST * 200);

    /// Value is measured with `measure_withdraw_test`
    /// Average gas for this method call don't exceed 3.4 `TGas`. 4 here just in case.
    pub(crate) const GAS_FOR_AFTER_WITHDRAW: Gas = tgas(4);
}

#[cfg(test)]
mod test {
    use crate::common::{
        gas_data::{GAS_FOR_AFTER_CLAIM, GAS_FOR_AFTER_WITHDRAW, GIGA},
        tgas,
    };

    #[test]
    fn test_gas_methods() {
        assert_eq!(tgas(50).0, 50_000_000_000_000);
        assert_eq!(GIGA, 1_000_000_000);
        assert_eq!(GAS_FOR_AFTER_CLAIM.0, 20_000_000_000_000);
        assert_eq!(GAS_FOR_AFTER_WITHDRAW.0, 4_000_000_000_000);
    }
}
