use std::ops::Mul;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

/// `UDecimal` represents a scientific representation of decimals.
///
/// The decimal number is represented in the form of `significand` divided by (10 raised to the power of `exponent`).
/// The `significand` and `exponent` are both positive integers.
/// The key components of this structure include:
///
/// * `significand`: The parts of the decimal number that holds significant digits, i.e., all digits including and
///                  following the leftmost nonzero digit.
///
/// * `exponent`: The part of the decimal number that represents the power to which 10 must be raised to yield the original number.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct UDecimal {
    pub significand: u128,
    pub exponent: u32,
}

impl UDecimal {
    /// Use this method only for View structures because
    /// it can cause a loss of precision
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn to_f32(&self) -> f32 {
        self.significand as f32 / 10u128.pow(self.exponent) as f32
    }
}

impl Mul<u128> for UDecimal {
    type Output = u128;

    fn mul(self, value: u128) -> Self::Output {
        value * self.significand / 10u128.pow(self.exponent)
    }
}

impl Mul<u128> for &UDecimal {
    type Output = u128;

    fn mul(self, value: u128) -> Self::Output {
        value * self.significand / 10u128.pow(self.exponent)
    }
}

impl UDecimal {
    pub(crate) fn new(significand: u128, exponent: u32) -> Self {
        Self { significand, exponent }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::udecimal::UDecimal;

    #[test]
    fn udecimal_to_f32() {
        let udecimal = UDecimal::new(12, 2);
        let float_value = udecimal.to_f32();

        assert_eq!(0.12, float_value);
    }

    #[test]
    fn udecimal_mul() {
        assert_eq!(UDecimal::new(12, 0) * 5, UDecimal::new(60, 0) * 1);
    }
}
