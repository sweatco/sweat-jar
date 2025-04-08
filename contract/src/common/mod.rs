pub(crate) mod test_data;
pub(crate) mod tests;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

pub mod gas_data {
    use near_sdk::Gas;

    /// Const of `ft_transfer` call in token contract
    pub(crate) const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(6);

    /// Const of after claim call with 1 jar
    const INITIAL_GAS_FOR_AFTER_CLAIM: Gas = Gas::from_tgas(4);

    /// Cost of adding 1 additional jar in after claim call. Measured with `measure_after_claim_total_test`
    const ADDITIONAL_AFTER_CLAIM_JAR_COST: Gas = Gas::from_ggas(80);

    /// Values are measured with `measure_after_claim_total_test`
    /// For now number of jars is arbitrary
    pub(crate) const GAS_FOR_AFTER_CLAIM: Gas =
        Gas::from_gas(INITIAL_GAS_FOR_AFTER_CLAIM.as_gas() + ADDITIONAL_AFTER_CLAIM_JAR_COST.as_gas() * 200);

    /// Value is measured with `measure_withdraw_test`
    /// Average gas for this method call don't exceed 3.4 `TGas`. 4 here just in case.
    pub(crate) const GAS_FOR_AFTER_WITHDRAW: Gas = Gas::from_tgas(4);

    #[cfg(not(test))]
    pub(crate) const GAS_FOR_AFTER_FEE_WITHDRAW: Gas = Gas::from_tgas(4);

    /// Value is measured with `measure_withdraw_all`
    /// 10 `TGas` was enough for 200 jars. 15 here just in case.
    pub(crate) const GAS_FOR_BULK_AFTER_WITHDRAW: Gas = Gas::from_tgas(15);
}

#[cfg(test)]
mod test {
    use crate::common::gas_data::{
        GAS_FOR_AFTER_CLAIM, GAS_FOR_AFTER_WITHDRAW, GAS_FOR_BULK_AFTER_WITHDRAW, GAS_FOR_FT_TRANSFER,
    };

    #[test]
    fn test_gas_methods() {
        assert_eq!(GAS_FOR_FT_TRANSFER.as_gas(), 6_000_000_000_000);
        assert_eq!(GAS_FOR_AFTER_CLAIM.as_gas(), 20_000_000_000_000);
        assert_eq!(GAS_FOR_AFTER_WITHDRAW.as_gas(), 4_000_000_000_000);
        assert_eq!(GAS_FOR_BULK_AFTER_WITHDRAW.as_gas(), 15_000_000_000_000);
    }
}
