use std::collections::HashMap;

use near_sdk::{env, json_types::U128, near_bindgen, require, AccountId};

use crate::{
    common::{TokenAmount, U32},
    event::{emit, EventKind, RestakeData},
    jar::view::{AggregatedTokenAmountView, JarIndexView, JarView},
    *,
};

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
    fn get_jar(&self, jar_index: JarIndexView) -> JarView;

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
    fn get_total_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView;

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
    fn get_principal(&self, jar_indices: Vec<JarIndexView>) -> AggregatedTokenAmountView;

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
    fn get_total_interest(&self, account_id: AccountId) -> AggregatedTokenAmountView;

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
    fn get_interest(&self, jar_indices: Vec<JarIndexView>) -> AggregatedTokenAmountView;

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
    fn restake(&mut self, jar_index: JarIndexView) -> JarView;
}

#[near_bindgen]
impl JarApi for Contract {
    fn get_jar(&self, jar_index: JarIndexView) -> JarView {
        self.get_jar_internal(jar_index.0).into()
    }

    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jar_ids(&account_id)
            .into_iter()
            .map(|index| self.get_jar(U32(index)))
            .collect()
    }

    fn get_total_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView {
        let jar_indices = self.account_jar_ids(&account_id).into_iter().map(U32).collect();

        self.get_principal(jar_indices)
    }

    fn get_principal(&self, jar_indices: Vec<JarIndexView>) -> AggregatedTokenAmountView {
        let mut detailed_amounts = HashMap::<JarIndexView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for index in jar_indices {
            let index = index.0;
            let principal = self.get_jar_internal(index).principal;

            detailed_amounts.insert(U32(index), U128(principal));
            total_amount += principal;
        }

        AggregatedTokenAmountView {
            detailed: detailed_amounts,
            total: U128(total_amount),
        }
    }

    fn get_total_interest(&self, account_id: AccountId) -> AggregatedTokenAmountView {
        let jar_indices = self.account_jar_ids(&account_id).into_iter().map(U32).collect();

        self.get_interest(jar_indices)
    }

    fn get_interest(&self, jar_indices: Vec<JarIndexView>) -> AggregatedTokenAmountView {
        let now = env::block_timestamp_ms();

        let mut detailed_amounts = HashMap::<JarIndexView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for index in jar_indices {
            let index = index.0;
            let jar = self.get_jar_internal(index);
            let interest = jar.get_interest(&self.get_product(&jar.product_id), now);

            detailed_amounts.insert(U32(index), U128(interest));
            total_amount += interest;
        }

        AggregatedTokenAmountView {
            detailed: detailed_amounts,
            total: U128(total_amount),
        }
    }

    fn restake(&mut self, jar_index: JarIndexView) -> JarView {
        let jar_index = jar_index.0;
        let jar = self.get_jar_internal(jar_index);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);

        let product = self.get_product(&jar.product_id);

        require!(product.allows_restaking(), "The product doesn't support restaking");
        require!(product.is_enabled, "The product is disabled");

        let now = env::block_timestamp_ms();
        require!(jar.is_liquidable(&product, now), "The jar is not mature yet");
        require!(!jar.is_empty(), "The jar is empty, nothing to restake");

        let index = self.jars.len() as JarIndex;
        let new_jar = Jar::create(
            index,
            jar.account_id.clone(),
            jar.product_id.clone(),
            jar.principal,
            now,
        );
        let withdraw_jar = jar.withdrawn(&product, jar.principal, now);

        self.save_jar(&account_id, &withdraw_jar);
        self.save_jar(&account_id, &new_jar);

        emit(EventKind::Restake(RestakeData {
            old_index: jar_index,
            new_index: new_jar.index,
        }));

        new_jar.into()
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::AccountId;

    use crate::{
        jar::model::Jar,
        product::tests::{get_product, YEAR_LOCKUP_TERM},
    };

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

        let interest = jar.get_interest(&product, YEAR_LOCKUP_TERM);
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
    use near_sdk::{
        json_types::{Base64VecU8, U128, U64},
        test_utils::accounts,
    };

    use crate::{
        common::{tests::Context, U32},
        jar::{api::JarApi, model::JarTicket},
        product::{
            api::*,
            command::RegisterProductCommand,
            tests::{
                get_register_premium_product_command, get_register_product_command,
                get_register_restakable_product_command,
            },
        },
        withdraw::api::WithdrawApi,
    };

    // Signature for structure (value -> utf8 bytes):
    // contract_id: "owner" -> [111, 119, 110, 101, 114]
    // account_id: "alice" -> [97, 108, 105, 99, 101]
    // product_id: "product_premium" -> [112, 114, 111, 100, 117, 99, 116, 95, 112, 114, 101, 109, 105, 117, 109]
    // amount: "1000000" -> [49, 48, 48, 48, 48, 48, 48]
    // last_jar_index: "" -> []
    // valid_until: "100000000" -> [49, 48, 48, 48, 48, 48, 48, 48, 48]
    // ***
    // result array: [111, 119, 110, 101, 114, 44, 97, 108, 105, 99, 101, 44, 112, 114, 111, 100, 117, 99, 116, 95, 112, 114, 101, 109, 105, 117, 109, 44, 49, 48, 48, 48, 48, 48, 48, 44, 48, 44, 49, 48, 48, 48, 48, 48, 48, 48, 48]
    // sha256(result array): [83, 24, 187, 67, 249, 130, 247, 51, 251, 43, 186, 72, 198, 208, 85, 25, 32, 170, 226, 43, 103, 129, 145, 210, 46, 38, 139, 38, 195, 50, 225, 87]
    // ***
    // Secret: [87, 86, 114, 129, 25, 247, 248, 94, 16, 119, 169, 202, 195, 11, 187, 107, 195, 182, 205, 70, 189, 120, 214, 228, 208, 115, 234, 0, 244, 21, 218, 113]
    // Pk: [33, 80, 163, 149, 64, 30, 150, 45, 68, 212, 97, 122, 213, 118, 189, 174, 239, 109, 48, 82, 50, 35, 197, 176, 50, 211, 183, 128, 207, 1, 8, 68]
    // ***
    // SIGNATURE: [46, 61, 129, 90, 99, 119, 184, 195, 18, 140, 235, 60, 165, 122, 106, 223, 196, 150, 246, 140, 9, 27, 49, 178, 65, 103, 187, 54, 81, 216, 150, 47, 137, 156, 91, 146, 224, 92, 40, 97, 85, 111, 3, 145, 61, 113, 135, 245, 201, 238, 91, 185, 179, 78, 174, 112, 138, 77, 68, 213, 136, 132, 20, 3]
    // base64(SIGNATURE): Lj2BWmN3uMMSjOs8pXpq38SW9owJGzGyQWe7NlHYli+JnFuS4FwoYVVvA5E9cYf1ye5bubNOrnCKTUTViIQUAw==
    pub fn get_valid_signature() -> Vec<u8> {
        vec![
            46, 61, 129, 90, 99, 119, 184, 195, 18, 140, 235, 60, 165, 122, 106, 223, 196, 150, 246, 140, 9, 27, 49,
            178, 65, 103, 187, 54, 81, 216, 150, 47, 137, 156, 91, 146, 224, 92, 40, 97, 85, 111, 3, 145, 61, 113, 135,
            245, 201, 238, 91, 185, 179, 78, 174, 112, 138, 77, 68, 213, 136, 132, 20, 3,
        ]
    }

    #[test]
    fn verify_ticket_with_valid_signature_and_date() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_premium_product_command(None))
        });

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context
            .contract
            .verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Invalid signature")]
    fn verify_ticket_with_invalid_signature() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_premium_product_command(None))
        });

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        let signature: Vec<u8> = vec![0, 1, 2];

        context
            .contract
            .verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Not matching signature")]
    fn verify_ticket_with_not_matching_signature() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(RegisterProductCommand {
                id: "another_product".to_string(),
                ..get_register_premium_product_command(None)
            })
        });

        let ticket = JarTicket {
            product_id: "another_product".to_string(),
            valid_until: U64(100000000),
        };

        let signature: Vec<u8> = [
            68, 119, 102, 0, 228, 169, 156, 208, 85, 165, 203, 130, 184, 28, 97, 129, 46, 72, 145, 7, 129, 127, 17, 57,
            153, 97, 38, 47, 101, 170, 168, 138, 44, 16, 162, 144, 53, 122, 188, 128, 118, 102, 133, 165, 195, 42, 182,
            22, 231, 99, 96, 72, 4, 79, 85, 143, 165, 10, 200, 44, 160, 90, 120, 14,
        ]
        .to_vec();

        context
            .contract
            .verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Ticket is outdated")]
    fn verify_ticket_with_invalid_date() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.set_block_timestamp_in_days(365);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_premium_product_command(None))
        });

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context
            .contract
            .verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Product product_premium doesn't exist")]
    fn verify_ticket_with_not_existing_product() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context
            .contract
            .verify(&admin, 1_000_000, &ticket, Some(Base64VecU8(get_valid_signature())));
    }

    #[test]
    #[should_panic(expected = "Signature is required")]
    fn verify_ticket_without_signature_when_required() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_premium_product_command(None))
        });

        let ticket = JarTicket {
            product_id: "product_premium".to_string(),
            valid_until: U64(100000000),
        };

        context.contract.verify(&admin, 1_000_000, &ticket, None);
    }

    #[test]
    fn verify_ticket_without_signature_when_not_required() {
        let admin = accounts(0);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

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
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice, ticket, U128(1_000_000), None);

        context.contract.restake(U32(0));
    }

    #[test]
    #[should_panic(expected = "The product doesn't support restaking")]
    fn restake_when_restaking_is_not_supported() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.switch_account(&alice);
        context.contract.restake(U32(0));
    }

    #[test]
    #[should_panic(expected = "The jar is not mature yet")]
    fn restake_before_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_restakable_product_command())
        });

        let ticket = JarTicket {
            product_id: "product_restakable".to_string(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.switch_account(&alice);
        context.contract.restake(U32(0));
    }

    #[test]
    #[should_panic(expected = "The product is disabled")]
    fn restake_with_disabled_product() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_restakable_product_command())
        });

        let ticket = JarTicket {
            product_id: "product_restakable".to_string(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .set_enabled(get_register_restakable_product_command().id, false)
        });

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(U32(0));
    }

    #[test]
    #[should_panic(expected = "The jar is empty, nothing to restake")]
    fn restake_empty_jar() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_restakable_product_command())
        });

        let ticket = JarTicket {
            product_id: "product_restakable".to_string(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.withdraw(U32(0), None);
        context.contract.restake(U32(0));
    }

    #[test]
    fn restake_after_maturity_for_restakable_product() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_restakable_product_command())
        });

        let ticket = JarTicket {
            product_id: "product_restakable".to_string(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(U32(0));

        let alice_jars = context.contract.get_jars_for_account(alice);
        assert_eq!(2, alice_jars.len());
        assert_eq!(0, alice_jars.iter().find(|item| item.index.0 == 0).unwrap().principal.0);
        assert_eq!(
            1_000_000,
            alice_jars.iter().find(|item| item.index.0 == 1).unwrap().principal.0
        );
    }

    #[test]
    #[should_panic(expected = "The product doesn't support restaking")]
    fn restake_after_maturity_for_not_restakable_product() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(U32(0));
    }

    #[test]
    #[should_panic(expected = "It's not possible to create new jars for this product")]
    fn create_jar_for_disabled_product() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(RegisterProductCommand {
                is_enabled: false,
                ..get_register_product_command()
            })
        });

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice, ticket, U128(1_000_000), None);
    }
}
