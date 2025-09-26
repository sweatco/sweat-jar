use crate::{DurationDays, Score};
use near_sdk::near;

pub type BoosterId = String;
pub type BoosterIndex = u8;

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct Booster {
    pub id: BoosterId,
    pub score: Score,
    pub duration: DurationDays,
}
