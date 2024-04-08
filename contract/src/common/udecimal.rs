use std::ops::Mul;

use near_sdk::near;

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
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct UDecimal {
    pub significand: u128,
    pub exponent: u32,
}

impl UDecimal {
    pub(crate) fn new(significand: u128, exponent: u32) -> Self {
        Self { significand, exponent }
    }

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
        (&self).mul(value)
    }
}

impl Mul<u128> for &UDecimal {
    type Output = u128;
    fn mul(self, value: u128) -> Self::Output {
        value * self.significand / 10u128.pow(self.exponent)
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
        assert_eq!(UDecimal::new(14, 1) * 10, UDecimal::new(14, 0) * 1);
        assert_eq!(UDecimal::new(16, 2) * 100, UDecimal::new(16, 0) * 1);
        assert_eq!(UDecimal::new(18, 3) * 1000, UDecimal::new(18, 0) * 1);
    }
}
