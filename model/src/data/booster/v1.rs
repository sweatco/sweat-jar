use crate::Score;
use near_sdk::near;

#[near]
#[derive(Clone, Debug)]
pub struct Booster {
    pub id: String,
    pub score: Score,
    pub duration: u16,
}
