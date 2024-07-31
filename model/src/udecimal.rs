use std::{
    cmp::max,
    ops::{Add, Mul},
};

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
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct UDecimal {
    pub significand: u128,
    pub exponent: u32,
}

impl UDecimal {
    pub const fn new(significand: u128, exponent: u32) -> Self {
        Self { significand, exponent }
    }

    /// Use this method only for View structures because
    /// it can cause a loss of precision
    #[allow(clippy::cast_precision_loss)]
    pub fn to_f32(self) -> f32 {
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

impl Mul<UDecimal> for UDecimal {
    type Output = UDecimal;

    fn mul(self, rhs: UDecimal) -> Self::Output {
        Self {
            significand: self.significand * rhs.significand,
            exponent: self.exponent + rhs.exponent,
        }
    }
}

impl Add for UDecimal {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let max_exponent = max(self.exponent, other.exponent);

        let adjust_significand = |mut significand: u128, exponent: u32| {
            for _ in 0..(max_exponent - exponent) {
                significand = significand.saturating_mul(10);
            }
            significand
        };

        let self_sig = adjust_significand(self.significand, self.exponent);
        let other_sig = adjust_significand(other.significand, other.exponent);

        UDecimal {
            significand: self_sig.saturating_add(other_sig),
            exponent: max_exponent,
        }
    }
}

#[cfg(test)]
mod tests {
    use fake::Fake;

    use crate::UDecimal;

    const MAX_EXPONENT: u32 = 6;

    #[test]
    fn udecimal_to_f32() {
        assert_eq!(0.1, UDecimal::new(1, 1).to_f32());
        assert_eq!(0.12, UDecimal::new(12, 2).to_f32());
        assert_eq!(1.0, UDecimal::new(1000, 3).to_f32());
        assert_eq!(5.0, UDecimal::new(50000, 4).to_f32());
    }

    #[test]
    fn udecimal_mul() {
        assert_eq!(UDecimal::new(12, 0) * 5, UDecimal::new(60, 0) * 1);
        assert_eq!(UDecimal::new(14, 1) * 10, UDecimal::new(14, 0) * 1);
        assert_eq!(UDecimal::new(16, 2) * 100, UDecimal::new(16, 0) * 1);
        assert_eq!(UDecimal::new(18, 3) * 1000, UDecimal::new(18, 0) * 1);

        for _ in 0..100_000 {
            let a = UDecimal::new((0..1000).fake(), (0..MAX_EXPONENT).fake());
            let b = UDecimal::new((0..1000).fake(), (0..MAX_EXPONENT).fake());

            let float_mul = a.to_f32() * b.to_f32();
            let decimal_mul = (a * b).to_f32();

            let diff = (float_mul - decimal_mul).abs();

            assert!(diff < 0.008, "Diff: {diff}");
        }
    }

    #[test]
    fn udecimal_add() {
        assert_eq!((UDecimal::new(5, 1) + UDecimal::new(3, 1)).to_f32(), 0.8);
        assert_eq!((UDecimal::new(3, 1) + UDecimal::new(5, 1)).to_f32(), 0.8);

        assert_eq!((UDecimal::new(5, 1) + UDecimal::new(3, 2)).to_f32(), 0.53);
        assert_eq!((UDecimal::new(3, 2) + UDecimal::new(5, 1)).to_f32(), 0.53);

        for _ in 0..100_000 {
            let a = UDecimal::new((0..1000).fake(), (0..MAX_EXPONENT).fake());
            let b = UDecimal::new((0..1000).fake(), (0..MAX_EXPONENT).fake());

            let float_add = a.to_f32() + b.to_f32();
            let decimal_add = (a + b).to_f32();

            let diff = (float_add - decimal_add).abs();

            assert!(diff < 0.00008, "Diff: {diff}");
        }
    }
}
