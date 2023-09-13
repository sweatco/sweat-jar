use std::collections::HashMap;

use near_sdk::{env, json_types::U128, near_bindgen, require, AccountId};

use crate::{
    assert_ownership,
    common::{u32::U32, TokenAmount},
    event::{emit, EventKind, RestakeData},
    jar::view::{AggregatedTokenAmountView, JarIndexView, JarView},
    Contract, ContractExt, Jar, JarIndex,
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
    use near_sdk::test_utils::accounts;

    use crate::{
        common::udecimal::UDecimal,
        jar::model::Jar,
        product::{
            model::{Apy, Product},
            tests::YEAR_IN_MS,
        },
    };

    #[test]
    fn get_interest_before_maturity() {
        let product = Product::generate("product")
            .apy(Apy::Constant(UDecimal::new(12, 2)))
            .lockup_term(2 * YEAR_IN_MS);
        let jar = Jar::generate(0, &accounts(0), &product.id).principal(100_000_000);

        let interest = jar.get_interest(&product, YEAR_IN_MS);
        assert_eq!(12_000_000, interest);
    }

    #[test]
    fn get_interest_after_maturity() {
        let product = Product::generate("product")
            .apy(Apy::Constant(UDecimal::new(12, 2)))
            .lockup_term(YEAR_IN_MS);
        let jar = Jar::generate(0, &accounts(0), &product.id).principal(100_000_000);

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
        common::{tests::Context, u32::U32, udecimal::UDecimal},
        jar::{
            api::JarApi,
            model::{Jar, JarTicket},
        },
        product::{
            api::*,
            helpers::MessageSigner,
            model::{Apy, DowngradableApy, Product},
            tests::YEAR_IN_MS,
        },
    };

    #[test]
    fn verify_ticket_with_valid_signature_and_date() {
        let admin = accounts(0);

        let signer = MessageSigner::new();
        let reference_product = generate_premium_product("premium_product", &signer);
        let context = Context::new(admin.clone()).with_products(&[reference_product.clone()]);

        let amount = 14_000_000;
        let ticket = JarTicket {
            product_id: reference_product.id,
            valid_until: U64(123000000),
        };

        let signature = signer.sign(context.get_signature_material(&admin, &ticket, amount).as_str());

        context
            .contract
            .verify(&admin, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Signature must be 64 bytes")]
    fn verify_ticket_with_invalid_signature() {
        let alice = accounts(0);
        let admin = accounts(1);

        let signer = MessageSigner::new();
        let reference_product = generate_premium_product("premium_product", &signer);
        let context = Context::new(admin).with_products(&[reference_product.clone()]);

        let amount = 1_000_000;
        let ticket = JarTicket {
            product_id: reference_product.id,
            valid_until: U64(100000000),
        };

        let signature: Vec<u8> = vec![0, 1, 2];

        context
            .contract
            .verify(&alice, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Not matching signature")]
    fn verify_ticket_with_not_matching_signature() {
        let admin = accounts(0);

        let signer = MessageSigner::new();
        let product = generate_premium_product("premium_product", &signer);
        let another_product = generate_premium_product("another_premium_product", &MessageSigner::new());

        let context = Context::new(admin.clone()).with_products(&[product, another_product.clone()]);

        let amount = 15_000_000;
        let ticket_for_another_product = JarTicket {
            product_id: another_product.id,
            valid_until: U64(100000000),
        };

        // signature made for wrong product
        let signature = signer.sign(
            context
                .get_signature_material(&admin, &ticket_for_another_product, amount)
                .as_str(),
        );

        context.contract.verify(
            &admin,
            amount,
            &ticket_for_another_product,
            Some(Base64VecU8(signature)),
        );
    }

    #[test]
    #[should_panic(expected = "Ticket is outdated")]
    fn verify_ticket_with_invalid_date() {
        let alice = accounts(0);
        let admin = accounts(1);

        let signer = MessageSigner::new();
        let reference_product = generate_premium_product("premium_product", &signer);
        let mut context = Context::new(admin).with_products(&[reference_product.clone()]);

        context.set_block_timestamp_in_days(365);

        let amount = 5_000_000;
        let ticket = JarTicket {
            product_id: reference_product.id,
            valid_until: U64(100000000),
        };

        let signature = signer.sign(context.get_signature_material(&alice, &ticket, amount).as_str());

        context
            .contract
            .verify(&alice, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Product not_existing_product doesn't exist")]
    fn verify_ticket_with_not_existing_product() {
        let admin = accounts(0);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        let signer = MessageSigner::new();
        let not_existing_product = generate_premium_product("not_existing_product", &signer);

        let amount = 500_000;
        let ticket = JarTicket {
            product_id: not_existing_product.id,
            valid_until: U64(100000000),
        };

        let signature = signer.sign(context.get_signature_material(&admin, &ticket, amount).as_str());

        context
            .contract
            .verify(&admin, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Signature is required")]
    fn verify_ticket_without_signature_when_required() {
        let admin = accounts(0);

        let signer = MessageSigner::new();
        let product = generate_premium_product("not_existing_product", &signer);
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 3_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(100000000),
        };

        context.contract.verify(&admin, amount, &ticket, None);
    }

    #[test]
    fn verify_ticket_without_signature_when_not_required() {
        let admin = accounts(0);

        let product = generate_product("regular_product");
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 4_000_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };

        context.contract.verify(&admin, amount, &ticket, None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn restake_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product").with_allows_restaking(true);
        let alice_jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin.clone())
            .with_products(&[product])
            .with_jars(&[alice_jar.clone()]);

        context.switch_account(&admin);
        context.contract.restake(U32(alice_jar.index));
    }

    #[test]
    #[should_panic(expected = "The product doesn't support restaking")]
    fn restake_when_restaking_is_not_supported() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("not_restakable_product").with_allows_restaking(false);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.switch_account(&alice);
        context.contract.restake(U32(jar.index));
    }

    #[test]
    #[should_panic(expected = "The jar is not mature yet")]
    fn restake_before_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product").with_allows_restaking(true);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.switch_account(&alice);
        context.contract.restake(U32(jar.index));
    }

    #[test]
    #[should_panic(expected = "The product is disabled")]
    fn restake_with_disabled_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product").with_allows_restaking(true);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin.clone())
            .with_products(&[product.clone()])
            .with_jars(&[jar.clone()]);

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| context.contract.set_enabled(product.id, false));

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(U32(jar.index));
    }

    #[test]
    #[should_panic(expected = "The jar is empty, nothing to restake")]
    fn restake_empty_jar() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product")
            .lockup_term(YEAR_IN_MS)
            .with_allows_restaking(true);
        let jar = Jar::generate(0, &alice, &product.id).principal(0);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(U32(jar.index));
    }

    #[test]
    fn restake_after_maturity_for_restakable_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product")
            .with_allows_restaking(true)
            .lockup_term(YEAR_IN_MS);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(U32(jar.index));

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

        let reference_product = generate_product("not_restakable_product").with_allows_restaking(false);
        let jar = Jar::generate(0, &alice, &reference_product.id).principal(1_000_000);
        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product.clone()])
            .with_jars(&[jar.clone()]);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract.restake(U32(jar.index));
    }

    #[test]
    #[should_panic(expected = "It's not possible to create new jars for this product")]
    fn create_jar_for_disabled_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product").enabled(false);
        let mut context = Context::new(admin).with_products(&[product.clone()]);

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context.contract.create_jar(alice, ticket, U128(1_000_000), None);
    }

    fn generate_premium_product(id: &str, signer: &MessageSigner) -> Product {
        Product::generate(id)
            .enabled(true)
            .public_key(signer.public_key())
            .cap(0, 100_000_000_000)
            .apy(Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }))
    }

    fn generate_product(id: &str) -> Product {
        Product::generate(id)
            .enabled(true)
            .cap(0, 100_000_000_000)
            .apy(Apy::Constant(UDecimal::new(20, 2)))
    }
}
