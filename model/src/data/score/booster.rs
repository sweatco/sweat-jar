use near_sdk::near;

use super::Score;

/// A type describing a boosted score with claim state.
///
/// The underlying `u16` is a bit-packed value where:
/// - The least significant bit (bit 0) represents whether the booster was claimed (1) or not (0)
/// - The remaining 15 bits contain the score value normalized by dividing by 10
///
/// This allows storing both the score and claim state in a single 16-bit value efficiently.
#[near(serializers=[borsh, json])]
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct BoostedScore(u16);

impl BoostedScore {
    /// Creates a new `BoostedScore` with the given score and claim state.
    ///
    /// # Arguments
    /// * `score` - The score value to store (will be normalized by dividing by 10)
    /// * `is_claimed` - Whether the booster has been claimed
    ///
    /// # Examples
    /// ```
    /// let booster = BoostedScore::new(1234, false);
    /// assert_eq!(booster.get_value(), 1234);
    /// assert!(!booster.is_claimed());
    /// ```
    pub fn new(score: Score, is_claimed: bool) -> Self {
        let normalized_score = score / 10;
        let claimed_bit = if is_claimed { 1 } else { 0 };
        Self((normalized_score << 1) | claimed_bit)
    }

    pub fn get_raw_value(&self) -> u16 {
        self.0
    }

    /// Returns whether the booster has been claimed.
    ///
    /// # Returns
    /// `true` if the booster was claimed, `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// let mut booster = BoostedScore::new(1000, false);
    /// assert!(!booster.is_claimed());
    ///
    /// booster.set_claimed(true);
    /// assert!(booster.is_claimed());
    /// ```
    pub fn is_claimed(&self) -> bool {
        (self.0 & 1) == 1
    }

    /// Returns the score value stored in this booster.
    ///
    /// # Returns
    /// The score value (denormalized by multiplying by 10).
    ///
    /// # Examples
    /// ```
    /// let booster = BoostedScore::new(1234, true);
    /// assert_eq!(booster.get_value(), 1234);
    /// ```
    pub fn get_value(&self) -> Score {
        let percent_as_int = self.0 >> 1;
        percent_as_int * 10
    }
    /// Claims the booster and returns its score value.
    ///
    /// This is a convenient method that combines checking if the booster is already claimed
    /// and claiming it in a single operation. If the booster is already claimed, it returns 0.
    /// If the booster is not yet claimed, it marks it as claimed and returns the score value.
    ///
    /// # Returns
    /// * The score value if the booster was successfully claimed (was not previously claimed)
    /// * `0` if the booster was already claimed
    ///
    /// # Examples
    /// ```
    /// let mut booster = BoostedScore::new(1500, false);
    /// assert!(!booster.is_claimed());
    ///
    /// // First claim returns the score
    /// let score = booster.claim();
    /// assert_eq!(score, 1500);
    /// assert!(booster.is_claimed());
    ///
    /// // Subsequent claims return 0
    /// let score_again = booster.claim();
    /// assert_eq!(score_again, 0);
    /// assert!(booster.is_claimed());
    /// ```
    pub fn claim(&mut self) -> Score {
        if self.is_claimed() {
            return 0;
        }

        self.set_claimed(true);

        self.get_value()
    }

    /// Sets the claim state of the booster.
    ///
    /// # Arguments
    /// * `value` - `true` to mark as claimed, `false` to mark as unclaimed
    ///
    /// # Examples
    /// ```
    /// let mut booster = BoostedScore::new(1000, false);
    /// booster.set_claimed(true);
    /// assert!(booster.is_claimed());
    ///
    /// booster.set_claimed(false);
    /// assert!(!booster.is_claimed());
    /// ```
    pub fn set_claimed(&mut self, value: bool) {
        if value {
            self.0 |= 1;
        } else {
            self.0 &= !1;
        }
    }
}
