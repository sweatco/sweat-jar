use near_sdk::{
    collections::{LookupMap, UnorderedMap},
    env::panic_str,
    near, IntoStorageKey,
};
use sweat_jar_model::{
    data::booster::{Booster, BoosterId, BoosterIndex},
    Timestamp,
};

#[near]
pub struct Boosters {
    genesis_timestamp: Timestamp,
    index: LookupMap<BoosterId, BoosterIndex>,
    items: UnorderedMap<BoosterIndex, Booster>,
}

impl Boosters {
    pub fn new(
        genesis_timestamp: Timestamp,
        index_prefix: impl IntoStorageKey,
        items_prefix: impl IntoStorageKey,
    ) -> Self {
        Self {
            genesis_timestamp,
            index: LookupMap::new(index_prefix),
            items: UnorderedMap::new(items_prefix),
        }
    }

    pub fn add(&mut self, booster: &Booster) {
        let index: u8 = self
            .items
            .len()
            .try_into()
            .unwrap_or_else(|_| panic_str("Too many boosters registered"));

        self.index.insert(&booster.id, &index);
        self.items.insert(&index, booster);
    }

    pub fn get(&self, id: BoosterId) -> Booster {
        let index = self
            .index
            .get(&id)
            .unwrap_or_else(|| panic_str(format!("No Booster with id {id} found.").as_str()));

        self.items
            .get(&index)
            .unwrap_or_else(|| panic_str(format!("No Booster with id {id} found.").as_str()))
    }

    pub fn list(&self) -> Vec<Booster> {
        self.items.values().collect()
    }

    pub fn contains(&self, id: &BoosterId) -> bool {
        self.index.contains_key(id)
    }
}
