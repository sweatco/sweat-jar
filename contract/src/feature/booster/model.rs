use near_sdk::{
    collections::{LookupMap, UnorderedMap},
    env::panic_str,
    near,
};
use sweat_jar_model::data::booster::Booster;

#[near]
struct Boosters {
    index: LookupMap<String, u8>,
    items: UnorderedMap<u8, Booster>,
}

impl Boosters {
    fn add(&mut self, booster: &Booster) {
        let index: u8 = self.items.len() as _;
        self.index.insert(&booster.id, &index);
        self.items.insert(&index, booster);
    }

    fn get(&self, id: String) -> Booster {
        let index = self
            .index
            .get(&id)
            .unwrap_or_else(|| panic_str(format!("No Booster with id {id} found.").as_str()));

        self.items
            .get(&index)
            .unwrap_or_else(|| panic_str(format!("No Booster with id {id} found.").as_str()))
    }
}
