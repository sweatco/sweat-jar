use near_sdk::{AccountId, env, near_bindgen, require};
use near_sdk::json_types::U128;

use crate::*;
use crate::event::{emit, EventKind, RestakeData};
use crate::jar::view::JarView;

/// The `JarApi` trait defines methods for managing deposit jars and their associated data within the smart contract.
pub trait JarApi {
    /// Retrieves information about a specific deposit jar by its index.
    ///
    /// # Arguments
    ///
    /// * `jar_index` - The index of the deposit jar for which information is being retrieved.
    ///
    /// # Returns
    ///
    /// A `JarView` struct containing details about the specified deposit jar.
    fn get_jar(&self, jar_index: JarIndex) -> JarView;

    /// Retrieves information about all deposit jars associated with a given account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The `AccountId` of the account for which jar information is being retrieved.
    ///
    /// # Returns
    ///
    /// A `Vec<JarView>` containing details about all deposit jars belonging to the specified account.
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView>;

    /// Retrieves the total principal amount across all deposit jars for a provided account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The `AccountId` of the account for which the total principal is being retrieved.
    ///
    /// # Returns
    ///
    /// An `U128` representing the sum of principal amounts across all deposit jars for the specified account.
    /// Returns 0 if the account has no associated jars.
    fn get_total_principal(&self, account_id: AccountId) -> U128;

    /// Retrieves the principal amount for a specific set of deposit jars.
    ///
    /// # Arguments
    ///
    /// * `jar_indices` - A `Vec<JarIndex>` containing the indices of the deposit jars for which the
    ///                   principal is being retrieved.
    ///
    /// # Returns
    ///
    /// An `U128` representing the sum of principal amounts for the specified deposit jars.
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> U128;

    /// Retrieves the total interest amount across all deposit jars for a provided account.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The `AccountId` of the account for which the total interest is being retrieved.
    ///
    /// # Returns
    ///
    /// An `U128` representing the sum of interest amounts across all deposit jars for the specified account.
    /// Returns 0 if the account has no associated jars.
    fn get_total_interest(&self, account_id: AccountId) -> U128;

    /// Retrieves the interest amount for a specific set of deposit jars.
    ///
    /// # Arguments
    ///
    /// * `jar_indices` - A `Vec<JarIndex>` containing the indices of the deposit jars for which the
    ///                   interest is being retrieved.
    ///
    /// # Returns
    ///
    /// An `U128` representing the sum of interest amounts for the specified deposit jars.
    ///
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> U128;

    /// Restakes the contents of a specified deposit jar into a new jar.
    ///
    /// # Arguments
    ///
    /// * `jar_index` - The index of the deposit jar from which the restaking is being initiated.
    ///
    /// # Returns
    ///
    /// A `JarView` containing details about the new jar created as a result of the restaking.
    ///
    /// # Panics
    ///
    /// This function may panic under the following conditions:
    /// - If the product of the original jar does not support restaking.
    /// - If the function is called by an account other than the owner of the original jar.
    /// - If the original jar is not yet mature.
    fn restake(&mut self, jar_index: JarIndex) -> JarView;
}

#[near_bindgen]
impl JarApi for Contract {
    fn get_jar(&self, index: JarIndex) -> JarView {
        self.get_jar_internal(index).into()
    }

    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jar_ids(&account_id)
            .iter()
            .map(|index| self.get_jar(*index))
            .collect()
    }

    fn get_total_principal(&self, account_id: AccountId) -> U128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_principal(jar_indices)
    }

    // TODO: tests
    fn get_principal(&self, jar_indices: Vec<JarIndex>) -> U128 {
        let result = jar_indices
            .iter()
            .map(|index| self.get_jar_internal(*index).principal)
            .sum();

        U128(result)
    }

    fn get_total_interest(&self, account_id: AccountId) -> U128 {
        let jar_indices = self.account_jar_ids(&account_id);

        self.get_interest(jar_indices)
    }

    // TODO: tests
    fn get_interest(&self, jar_indices: Vec<JarIndex>) -> U128 {
        let now = env::block_timestamp_ms();
        let result = jar_indices
            .iter()
            .map(|index| self.get_jar_internal(*index))
            .map(|jar| jar.get_interest(&self.get_product(&jar.product_id), now))
            .sum();

        U128(result)
    }

    fn restake(&mut self, jar_index: JarIndex) -> JarView {
        let jar = self.get_jar_internal(jar_index);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);

        let product = self.get_product(&jar.product_id);

        require!(product.allows_restaking(), "The product doesn't support restaking");

        let now = env::block_timestamp_ms();
        require!(jar.is_liquidable(&product, now), "The jar is not mature yet");

        let index = self.jars.len() as JarIndex;
        let new_jar = Jar::create(index, jar.account_id.clone(), jar.product_id.clone(), jar.principal, now);
        let withdraw_jar = jar.withdrawn(&product, jar.principal, now);

        self.save_jar(&account_id, &withdraw_jar);
        self.save_jar(&account_id, &new_jar);

        emit(EventKind::Restake(RestakeData { old_index: index, new_index: new_jar.index }));

        new_jar.into()
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::AccountId;

    use crate::jar::model::Jar;
    use crate::product::tests::get_product;

    #[test]
    fn get_interest_before_maturity() {
        let product = get_product();
        let jar = Jar::create(
            0,
            AccountId::new_unchecked("alice".to_string()),
            product.clone().id,
            100_000_000,
            0,
        );

        let interest = jar.get_interest(&product, 365 * 24 * 60 * 60 * 1000);
        assert_eq!(12_000_000, interest);
    }

    #[test]
    fn get_interest_after_maturity() {
        let product = get_product();
        let jar = Jar::create(
            0,
            AccountId::new_unchecked("alice".to_string()),
            product.clone().id,
            100_000_000,
            0,
        );

        let interest = jar.get_interest(&product, 400 * 24 * 60 * 60 * 1000);
        assert_eq!(12_000_000, interest);
    }
}

#[cfg(test)]
mod signature_tests {
    use near_sdk::json_types::{Base64VecU8, U128, U64};
    use near_sdk::test_utils::accounts;

    use crate::common::tests::Context;
    use crate::jar::api::JarApi;
    use crate::jar::model::JarTicket;
    use crate::product::api::*;
    use crate::product::command::RegisterProductCommand;
    use crate::product::tests::{get_register_premium_product_command, get_register_product_command, get_register_restakable_product_command};

    // Signature for structure (value -> utf8 bytes):
    // contract_id: "owner" -> [111, 119, 110, 101, 114]
    // account_id: "alice" -> [97, 108, 105, 99, 101]
    // product_id: "product_premium" -> [112, 114, 111, 100, 117, 99, 116, 95, 112, 114, 101, 109, 105, 117, 109]
    // amount: "1000000" -> [49, 48, 48, 48, 48, 48, 48]
    // last_jar_index: "0" -> [48]
    // valid_until: "100000000" -> [49, 48, 48, 48, 48, 48, 48, 48, 48]
    // ***
    // result array: [111, 119, 110, 101, 114, 44, 97, 108, 105, 99, 101, 44, 112, 114, 111, 100, 117, 99, 116, 95, 112, 114, 101, 109, 105, 117, 109, 44, 49, 48, 48, 48, 48, 48, 48, 44, 48, 44, 49, 48, 48, 48, 48, 48, 48, 48, 48]
    // sha256(result array): [83, 24, 187, 67, 249, 130, 247, 51, 251, 43, 186, 72, 198, 208, 85, 25, 32, 170, 226, 43, 103, 129, 145, 210, 46, 38, 139, 38, 195, 50, 225, 87]
    // ***
    // Secret: [87, 86, 114, 129, 25, 247, 248, 94, 16, 119, 169, 202, 195, 11, 187, 107, 195, 182, 205, 70, 189, 120, 214, 228, 208, 115, 234, 0, 244, 21, 218, 113]
    // Pk: [33, 80, 163, 149, 64, 30, 150, 45, 68, 212, 97, 122, 213, 118, 189, 174, 239, 109, 48, 82, 50, 35, 197, 176, 50, 211, 183, 128, 207, 1, 8, 68]
    // ***
    // SIGNATURE: [106, 169, 28, 95, 190, 177, 11, 212, 73, 215, 174, 31, 143, 61, 191, 107, 132, 100, 38, 8, 90, 248, 246, 79, 84, 216, 122, 215, 182, 136, 134, 160, 3, 10, 118, 74, 123, 31, 91, 121, 192, 142, 25, 97, 54, 231, 253, 26, 239, 15, 24, 201, 110, 243, 6, 134, 246, 17, 148, 178, 251, 68, 57, 13]
    fn get_valid_signature() -> Vec<u8> {
        vec![
            106, 169, 28, 95, 190, 177, 11, 212, 73, 215, 174, 31, 143, 61, 191, 107, 132, 100, 38,
            8, 90, 248, 246, 79, 84, 216, 122, 215, 182, 136, 134, 160, 3, 10, 118, 74, 123, 31, 91,
            121, 192, 142, 25, 97, 54, 231, 253, 26, 239, 15, 24, 201, 110, 243, 6, 134, 246, 17,
            148, 178, 251, 68, 57, 13,
        ]
    }

    #[test]
    fn verify_ticket_with_valid_signature_and_date() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_premium_product_command());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context.contract.verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Invalid signature")]
    fn verify_ticket_with_invalid_signature() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_premium_product_command());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        let signature: Vec<u8> = vec![0, 1, 2];

        context.contract.verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Not matching signature")]
    fn verify_ticket_with_not_matching_signature() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);

        let command = RegisterProductCommand {
            id: "another_product".to_string(),
            ..get_register_premium_product_command()
        };
        context.contract.register_product(command);

        let ticket = JarTicket {
            product_id: "another_product".to_string(),
            valid_until: U64(100000000),
        };

        let signature: Vec<u8> = [
            68, 119, 102, 0, 228, 169, 156, 208, 85, 165, 203, 130, 184, 28, 97, 129, 46, 72, 145,
            7, 129, 127, 17, 57, 153, 97, 38, 47, 101, 170, 168, 138, 44, 16, 162, 144, 53, 122,
            188, 128, 118, 102, 133, 165, 195, 42, 182, 22, 231, 99, 96, 72, 4, 79, 85, 143, 165,
            10, 200, 44, 160, 90, 120, 14
        ].to_vec();

        context.contract.verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Ticket is outdated")]
    fn verify_ticket_with_invalid_date() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.set_block_timestamp_in_days(365);
        context.contract.register_product(get_register_premium_product_command());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context.contract.verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Product product_premium doesn't exist")]
    fn verify_ticket_with_not_existing_product() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context.contract.verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Signature is required")]
    fn verify_ticket_without_signature_when_required() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_premium_product_command());

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context.contract.verify(&admin, 1_000_000, &ticket, None);
    }

    #[test]
    fn verify_ticket_without_signature_when_not_required() {
        let admin = accounts(0);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };

        context.contract.verify(&admin, 1_000_000, &ticket, None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn restake_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice, ticket, U128(1_000_000), None);

        context.contract.restake(0);
    }

    #[test]
    #[should_panic(expected = "The product doesn't support restaking")]
    fn restake_when_restaking_is_not_supported() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.switch_account(&alice);
        context.contract.restake(0);
    }

    #[test]
    #[should_panic(expected = "The jar is not mature yet")]
    fn restake_before_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_restakable_product_command());

        let ticket = JarTicket {
            product_id: "product_restakable".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.switch_account(&alice);
        context.contract.restake(0);
    }

    #[test]
    fn restake_after_maturity_for_restakable_product() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_restakable_product_command());

        let ticket = JarTicket {
            product_id: "product_restakable".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(0);

        let alice_jars = context.contract.get_jars_for_account(alice);
        assert_eq!(2, alice_jars.len());
        assert_eq!(0, alice_jars.iter().find(|item| item.index == 0).unwrap().principal.0);
        assert_eq!(1_000_000, alice_jars.iter().find(|item| item.index == 1).unwrap().principal.0);
    }

    #[test]
    #[should_panic(expected = "The product doesn't support restaking")]
    fn restake_after_maturity_for_not_restakable_product() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(0);
    }
}