use near_sdk::Gas;

pub(crate) mod tests;
pub(crate) mod u32;
pub(crate) mod udecimal;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

/// Amount of fungible tokens
pub type TokenAmount = u128;

pub(crate) const MS_IN_SECOND: u64 = 1000;
pub(crate) const MS_IN_MINUTE: u64 = MS_IN_SECOND * 60;
pub(crate) const MINUTES_IN_YEAR: u64 = 365 * 24 * 60;
pub(crate) const MS_IN_YEAR: Duration = MINUTES_IN_YEAR * MS_IN_MINUTE;

const TERA: u64 = Gas::ONE_TERA.0;
const GIGA: u64 = TERA / 1000;

pub const fn tgas(val: u64) -> Gas {
    Gas(TERA * val)
}

/// Const of after claim call with 1 jar
const INITIAL_GAS_FOR_AFTER_CLAIM: u64 = 4 * TERA;

/// Cost of adding 1 additional jar in after claim call. Measured with `measure_after_claim_total_test`
const ADDITIONAL_AFTER_CLAIM_JAR_COST: u64 = 300 * GIGA;

/// Values are measured with `measure_after_claim_total_test`
/// For now number of jars is arbitrary
pub(crate) const GAS_FOR_AFTER_CLAIM: Gas = Gas(INITIAL_GAS_FOR_AFTER_CLAIM + ADDITIONAL_AFTER_CLAIM_JAR_COST * 50);

/// Value is measured with `measure_withdraw_test`
/// Average gas for this method call don't exceed 3.4 `TGas`. 4 here just in case.
pub(crate) const GAS_FOR_AFTER_WITHDRAW: Gas = tgas(4);

#[cfg(test)]
mod test {
    use crate::common::tgas;

    #[test]
    fn test_gas_methods() {
        assert_eq!(tgas(50).0, 50_000_000_000_000);
    }
}
