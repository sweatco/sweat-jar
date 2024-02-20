#![cfg(feature = "integration-test")]

use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    log, near_bindgen,
    store::LookupMap,
};
use sweat_jar_model::api::TestIncreasedEnumSize;

use crate::{Contract, ContractExt};

#[derive(BorshSerialize, BorshDeserialize)]
enum SmallEnum {
    Smol(u8),
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
enum BigEnum {
    Smol(u8),
    Big([u8; 32]),
}

const KEY: &str = "BIG_SMALL_ENUM_STORAGE";

#[near_bindgen]
impl TestIncreasedEnumSize for Contract {
    fn store_small_enum(&mut self) {
        let mut map = LookupMap::<u8, SmallEnum>::new(KEY.as_bytes());

        map.insert(0, SmallEnum::Smol(0));
        map.insert(1, SmallEnum::Smol(1));
        map.insert(2, SmallEnum::Smol(2));

        log!("store_small_enum: OK");
    }

    fn migrate_to_big_enum(&mut self) {
        let mut map = LookupMap::<u8, BigEnum>::new(KEY.as_bytes());

        map.insert(2, BigEnum::Smol(55));

        map.insert(3, BigEnum::Smol(3));
        map.insert(4, BigEnum::Smol(4));
        map.insert(5, BigEnum::Smol(5));

        map.insert(6, BigEnum::Big([6; 32]));

        log!("migrate_to_big_enum: OK");
    }

    fn check_big_enum(&mut self) {
        let map = LookupMap::<u8, BigEnum>::new(KEY.as_bytes());

        assert_eq!(map.get(&0).unwrap(), &BigEnum::Smol(0));
        assert_eq!(map.get(&1).unwrap(), &BigEnum::Smol(1));
        assert_eq!(map.get(&2).unwrap(), &BigEnum::Smol(55));

        assert_eq!(map.get(&3).unwrap(), &BigEnum::Smol(3));
        assert_eq!(map.get(&4).unwrap(), &BigEnum::Smol(4));
        assert_eq!(map.get(&5).unwrap(), &BigEnum::Smol(5));

        assert_eq!(map.get(&6).unwrap(), &BigEnum::Big([6; 32]));

        log!("big: {:?}", map.get(&6).unwrap());

        log!("check_big_enum: OK");
    }
}
