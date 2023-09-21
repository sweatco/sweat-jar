use std::collections::HashMap;

use near_sdk::{env, json_types::U128, near_bindgen, require, AccountId};

use crate::{
    assert_ownership,
    common::{u32::U32, TokenAmount},
    event::{emit, EventKind, RestakeData},
    jar::view::{AggregatedInterestView, AggregatedTokenAmountView, JarIDView, JarView},
    Contract, ContractExt, Jar,
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
    fn get_jar(&self, account_id: AccountId, jar_index: JarIDView) -> JarView;

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
    fn get_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView;

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
    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView;

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
    fn get_interest(&self, account_id: AccountId) -> AggregatedInterestView;

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
    fn restake(&mut self, account_id: AccountId, jar_index: JarIDView) -> JarView;
}

#[near_bindgen]
impl JarApi for Contract {
    fn get_jar(&self, account_id: AccountId, jar_id: JarIDView) -> JarView {
        self.get_jar_internal(&account_id, jar_id.0).into()
    }

    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jars(&account_id).iter().map(Into::into).collect()
    }

    fn get_total_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView {
        self.get_principal(account_id)
    }

    fn get_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView {
        let mut detailed_amounts = HashMap::<JarIDView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for jar in self.get_jars_for_account(account_id) {
            let index = jar.id;
            let principal = jar.principal;

            detailed_amounts.insert(index, principal);
            total_amount += principal.0;
        }

        AggregatedTokenAmountView {
            detailed: detailed_amounts,
            total: U128(total_amount),
        }
    }

    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        self.get_interest(account_id)
    }

    fn get_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        let now = env::block_timestamp_ms();

        let mut detailed_amounts = HashMap::<JarIDView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for jar in self.account_jars(&account_id) {
            let index = jar.id;
            let interest = jar.get_interest(self.get_product(&jar.product_id), now);

            detailed_amounts.insert(U32(index), U128(interest));
            total_amount += interest;
        }

        AggregatedInterestView {
            amount: AggregatedTokenAmountView {
                detailed: detailed_amounts,
                total: U128(total_amount),
            },
            timestamp: now,
        }
    }

    fn restake(&mut self, account_id: AccountId, jar_index: JarIDView) -> JarView {
        let jar_index = jar_index.0;
        let jar = self.get_jar_internal(&account_id, jar_index);
        let account_id = env::predecessor_account_id();

        assert_ownership(jar, &account_id);

        let product = self.get_product(&jar.product_id);

        require!(product.allows_restaking(), "The product doesn't support restaking");
        require!(product.is_enabled, "The product is disabled");

        let now = env::block_timestamp_ms();
        require!(jar.is_liquidable(product, now), "The jar is not mature yet");
        require!(!jar.is_empty(), "The jar is empty, nothing to restake");

        let principal = jar.principal;

        let id = self.last_jar_id + 1;
        let new_jar = Jar::create(id, jar.account_id.clone(), jar.product_id.clone(), principal, now);
        let (should_be_closed, withdraw_jar) = jar.withdrawn(product, principal, now);

        if should_be_closed {
            self.delete_jar(withdraw_jar);
        } else {
            self.save_jar(&account_id, withdraw_jar);
        }

        self.save_jar(&account_id, new_jar.clone());

        self.increment_jar_id();

        emit(EventKind::Restake(RestakeData {
            old_index: jar_index,
            new_index: new_jar.id,
        }));

        new_jar.into()
    }
}
