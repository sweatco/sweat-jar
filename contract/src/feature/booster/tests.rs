#![cfg(test)]

use near_sdk::AccountId;
use rstest::rstest;
use sweat_jar_model::{api::BoosterApi, data::booster::Booster};

use crate::common::testing::{accounts::*, Context};
use crate::feature::booster::model::Boosters;
use crate::StorageKey;

// Test utilities for creating booster test data
fn create_booster(id: &str, score: u16, duration: u16) -> Booster {
    Booster {
        id: id.to_string(),
        score,
        duration,
    }
}

fn create_test_boosters() -> Boosters {
    Boosters::new(0, StorageKey::BoostersIndex, StorageKey::BoostersItems)
}

mod model_tests {
    use super::*;

    #[rstest]
    fn test_new_boosters() {
        let boosters = create_test_boosters();

        // Verify that a new Boosters instance is created with empty collections
        assert_eq!(boosters.list().len(), 0);
    }

    #[rstest]
    fn test_add_single_booster() {
        let mut boosters = create_test_boosters();
        let booster = create_booster("booster_1", 100, 30);

        boosters.add(&booster);

        // Verify booster was added
        assert_eq!(boosters.list().len(), 1);
        assert!(boosters.contains(&booster.id));

        // Verify the booster can be retrieved
        let retrieved_booster = boosters.get(booster.id.clone());
        assert_eq!(retrieved_booster.id, booster.id);
        assert_eq!(retrieved_booster.score, booster.score);
        assert_eq!(retrieved_booster.duration, booster.duration);
    }

    #[rstest]
    fn test_add_multiple_boosters() {
        let mut boosters = create_test_boosters();
        let booster1 = create_booster("booster_1", 100, 30);
        let booster2 = create_booster("booster_2", 200, 60);
        let booster3 = create_booster("booster_3", 300, 90);

        boosters.add(&booster1);
        boosters.add(&booster2);
        boosters.add(&booster3);

        // Verify all boosters were added
        assert_eq!(boosters.list().len(), 3);
        assert!(boosters.contains(&booster1.id));
        assert!(boosters.contains(&booster2.id));
        assert!(boosters.contains(&booster3.id));

        // Verify each booster can be retrieved correctly
        let retrieved1 = boosters.get(booster1.id.clone());
        assert_eq!(retrieved1.score, 100);
        assert_eq!(retrieved1.duration, 30);

        let retrieved2 = boosters.get(booster2.id.clone());
        assert_eq!(retrieved2.score, 200);
        assert_eq!(retrieved2.duration, 60);

        let retrieved3 = boosters.get(booster3.id.clone());
        assert_eq!(retrieved3.score, 300);
        assert_eq!(retrieved3.duration, 90);
    }

    #[rstest]
    fn test_get_existing_booster() {
        let mut boosters = create_test_boosters();
        let booster = create_booster("test_booster", 150, 45);

        boosters.add(&booster);
        let retrieved = boosters.get(booster.id.clone());

        assert_eq!(retrieved.id, booster.id);
        assert_eq!(retrieved.score, booster.score);
        assert_eq!(retrieved.duration, booster.duration);
    }

    #[rstest]
    #[should_panic(expected = "No Booster with id nonexistent_booster found.")]
    fn test_get_nonexistent_booster() {
        let boosters = create_test_boosters();
        boosters.get("nonexistent_booster".to_string());
    }

    #[rstest]
    fn test_list_all_boosters() {
        let mut boosters = create_test_boosters();

        // Initially empty
        assert_eq!(boosters.list().len(), 0);

        // Add multiple boosters
        let booster1 = create_booster("booster_1", 100, 30);
        let booster2 = create_booster("booster_2", 200, 60);
        let booster3 = create_booster("booster_3", 300, 90);

        boosters.add(&booster1);
        boosters.add(&booster2);
        boosters.add(&booster3);

        // Verify all boosters are returned
        let boosters_list = boosters.list();
        assert_eq!(boosters_list.len(), 3);

        // Verify the boosters are in the list (order may vary)
        let ids: Vec<String> = boosters_list.iter().map(|b| b.id.clone()).collect();
        assert!(ids.contains(&booster1.id));
        assert!(ids.contains(&booster2.id));
        assert!(ids.contains(&booster3.id));
    }

    #[rstest]
    fn test_contains_existing_booster() {
        let mut boosters = create_test_boosters();
        let booster = create_booster("test_booster", 100, 30);

        // Initially should not contain the booster
        assert!(!boosters.contains(&booster.id));

        boosters.add(&booster);

        // After adding, should contain the booster
        assert!(boosters.contains(&booster.id));
    }

    #[rstest]
    fn test_contains_nonexistent_booster() {
        let boosters = create_test_boosters();

        // Should not contain non-existent booster
        assert!(!boosters.contains(&"nonexistent_booster".to_string()));
    }

    #[rstest]
    fn test_booster_indexing() {
        let mut boosters = create_test_boosters();

        // Add boosters in a specific order
        let booster1 = create_booster("first_booster", 100, 30);
        let booster2 = create_booster("second_booster", 200, 60);
        let booster3 = create_booster("third_booster", 300, 90);

        boosters.add(&booster1);
        boosters.add(&booster2);
        boosters.add(&booster3);

        // Verify that each booster can be retrieved by its ID
        // This tests the internal indexing mechanism
        let retrieved1 = boosters.get(booster1.id.clone());
        let retrieved2 = boosters.get(booster2.id.clone());
        let retrieved3 = boosters.get(booster3.id.clone());

        assert_eq!(retrieved1.id, booster1.id);
        assert_eq!(retrieved2.id, booster2.id);
        assert_eq!(retrieved3.id, booster3.id);

        // Verify that the boosters maintain their original data
        assert_eq!(retrieved1.score, 100);
        assert_eq!(retrieved1.duration, 30);
        assert_eq!(retrieved2.score, 200);
        assert_eq!(retrieved2.duration, 60);
        assert_eq!(retrieved3.score, 300);
        assert_eq!(retrieved3.duration, 90);
    }

    #[rstest]
    fn test_booster_with_edge_case_values() {
        let mut boosters = create_test_boosters();

        // Test with minimum values
        let min_booster = create_booster("min_booster", 0, 0);
        boosters.add(&min_booster);

        // Test with maximum values
        let max_booster = create_booster("max_booster", u16::MAX, u16::MAX);
        boosters.add(&max_booster);

        // Test with special characters in ID
        let special_booster = create_booster("booster-with_special.chars", 500, 120);
        boosters.add(&special_booster);

        assert_eq!(boosters.list().len(), 3);

        // Verify all boosters can be retrieved
        let retrieved_min = boosters.get(min_booster.id.clone());
        assert_eq!(retrieved_min.score, 0);
        assert_eq!(retrieved_min.duration, 0);

        let retrieved_max = boosters.get(max_booster.id.clone());
        assert_eq!(retrieved_max.score, u16::MAX);
        assert_eq!(retrieved_max.duration, u16::MAX);

        let retrieved_special = boosters.get(special_booster.id.clone());
        assert_eq!(retrieved_special.score, 500);
        assert_eq!(retrieved_special.duration, 120);
    }

    #[rstest]
    #[should_panic(expected = "Too many boosters registered")]
    fn test_boosters_list_overflow_handling() {
        let mut boosters = create_test_boosters();

        for i in 0..=(u8::MAX as u16 + 1) as usize {
            let booster = create_booster(&format!("booster_{}", i), i as u16, 30);
            boosters.add(&booster);
        }
    }
}

mod api_tests {
    use super::*;

    #[rstest]
    fn test_register_booster_by_manager(admin: AccountId) {
        let mut context = Context::new(admin.clone());
        let booster = create_booster("test_booster", 100, 30);

        context.switch_account_to_manager();
        context.contract().register_booster(booster.clone());

        // Verify booster was registered
        let boosters = context.contract().get_boosters();
        assert_eq!(boosters.len(), 1);
        assert_eq!(boosters[0].id, booster.id);
        assert_eq!(boosters[0].score, booster.score);
        assert_eq!(boosters[0].duration, booster.duration);
    }

    #[rstest]
    #[should_panic(expected = "Can be performed only by admin")]
    fn test_register_booster_by_not_manager(admin: AccountId, alice: AccountId) {
        let mut context = Context::new(admin);
        let booster = create_booster("test_booster", 100, 30);

        context.switch_account(&alice);
        context.contract().register_booster(booster);
    }

    #[rstest]
    #[should_panic(expected = "Booster test_booster is already registered")]
    fn test_register_duplicate_booster(admin: AccountId) {
        let mut context = Context::new(admin.clone());
        let booster = create_booster("test_booster", 100, 30);

        context.switch_account_to_manager();
        context.contract().register_booster(booster.clone());

        // Try to register the same booster again
        context.contract().register_booster(booster);
    }

    #[rstest]
    fn test_get_boosters_when_empty(admin: AccountId) {
        let context = Context::new(admin);

        let boosters = context.contract().get_boosters();
        assert_eq!(boosters.len(), 0);
    }

    #[rstest]
    fn test_get_boosters_with_multiple_boosters(admin: AccountId) {
        let mut context = Context::new(admin.clone());

        let booster1 = create_booster("booster_1", 100, 30);
        let booster2 = create_booster("booster_2", 200, 60);
        let booster3 = create_booster("booster_3", 300, 90);

        context.switch_account_to_manager();
        context.contract().register_booster(booster1.clone());
        context.contract().register_booster(booster2.clone());
        context.contract().register_booster(booster3.clone());

        let boosters = context.contract().get_boosters();
        assert_eq!(boosters.len(), 3);

        // Verify all boosters are present (order may vary)
        let ids: Vec<String> = boosters.iter().map(|b| b.id.clone()).collect();
        assert!(ids.contains(&booster1.id));
        assert!(ids.contains(&booster2.id));
        assert!(ids.contains(&booster3.id));

        // Verify booster data integrity
        for booster in &boosters {
            match booster.id.as_str() {
                "booster_1" => {
                    assert_eq!(booster.score, 100);
                    assert_eq!(booster.duration, 30);
                }
                "booster_2" => {
                    assert_eq!(booster.score, 200);
                    assert_eq!(booster.duration, 60);
                }
                "booster_3" => {
                    assert_eq!(booster.score, 300);
                    assert_eq!(booster.duration, 90);
                }
                _ => panic!("Unexpected booster ID: {}", booster.id),
            }
        }
    }

    #[rstest]
    fn test_register_multiple_boosters_and_retrieve(admin: AccountId) {
        let mut context = Context::new(admin.clone());

        // Register multiple boosters with different characteristics
        let boosters_to_register = vec![
            create_booster("min_booster", 0, 0),
            create_booster("max_booster", u16::MAX, u16::MAX),
            create_booster("normal_booster", 500, 120),
            create_booster("special_chars_booster", 1000, 365),
        ];

        context.switch_account_to_manager();

        // Register all boosters
        for booster in &boosters_to_register {
            context.contract().register_booster(booster.clone());
        }

        // Retrieve all boosters
        let retrieved_boosters = context.contract().get_boosters();
        assert_eq!(retrieved_boosters.len(), 4);

        // Verify each booster can be found
        for original_booster in &boosters_to_register {
            let found_booster = retrieved_boosters
                .iter()
                .find(|b| b.id == original_booster.id)
                .expect(&format!("Booster {} not found", original_booster.id));

            assert_eq!(found_booster.id, original_booster.id);
            assert_eq!(found_booster.score, original_booster.score);
            assert_eq!(found_booster.duration, original_booster.duration);
        }
    }

    #[rstest]
    fn test_register_booster_with_edge_case_values(admin: AccountId) {
        let mut context = Context::new(admin.clone());

        // Test with minimum values
        let min_booster = create_booster("min_booster", 0, 0);
        context.switch_account_to_manager();
        context.contract().register_booster(min_booster.clone());

        // Test with maximum values
        let max_booster = create_booster("max_booster", u16::MAX, u16::MAX);
        context.contract().register_booster(max_booster.clone());

        // Test with special characters in ID
        let special_booster = create_booster("booster-with_special.chars", 500, 120);
        context.contract().register_booster(special_booster.clone());

        let boosters = context.contract().get_boosters();
        assert_eq!(boosters.len(), 3);

        // Verify all boosters were registered correctly
        let min_found = boosters.iter().find(|b| b.id == min_booster.id).unwrap();
        assert_eq!(min_found.score, 0);
        assert_eq!(min_found.duration, 0);

        let max_found = boosters.iter().find(|b| b.id == max_booster.id).unwrap();
        assert_eq!(max_found.score, u16::MAX);
        assert_eq!(max_found.duration, u16::MAX);

        let special_found = boosters.iter().find(|b| b.id == special_booster.id).unwrap();
        assert_eq!(special_found.score, 500);
        assert_eq!(special_found.duration, 120);
    }
}
