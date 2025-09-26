use crate::{DurationDays, Score};
use near_sdk::near;

pub type BoosterId = String;

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct Booster {
    pub id: BoosterId,
    pub score: Score,
    pub duration: DurationDays,
}
