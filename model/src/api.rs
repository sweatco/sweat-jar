use integration_trait::make_integration_version;
use near_sdk::{
    json_types::{Base64VecU8, U128},
    AccountId, PromiseOrValue,
};

use crate::{
    claimed_amount_view::ClaimedAmountView,
    jar::{AggregatedInterestView, AggregatedTokenAmountView, CeFiJar, JarIdView, JarView},
    product::{ProductView, RegisterProductCommand},
    withdraw::WithdrawView,
    ProductId,
};

#[cfg(feature = "integration-test")]
pub struct SweatJarContract<'a> {
    pub contract: &'a near_workspaces::Contract,
}

#[make_integration_version]
pub trait InitApi {
    fn init(token_account_id: AccountId, fee_account_id: AccountId, manager: AccountId) -> Self;
}

/// The `ClaimApi` trait defines methods for claiming interest from jars within the smart contract.
#[make_integration_version]
pub trait ClaimApi {
    /// Claims all available interest from all deposit jars belonging to the calling account.
    ///
    /// * `detailed` – An optional boolean value specifying if the method must return only total amount of claimed tokens
    ///                or detailed summary for each claimed jar. Set it `true` to get a detailed result. In case of `false`
    ///                or `None` it returns only the total claimed amount.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<ClaimedAmountView>` representing the amount of tokens claimed
    /// and probably a map containing amount of tokens claimed from each Jar. If the total available
    /// interest across all jars is zero, the returned value will also be zero and the detailed map will be empty (if requested).
    fn claim_total(&mut self, detailed: Option<bool>) -> PromiseOrValue<ClaimedAmountView>;

    /// Claims interest from specific deposit jars with provided IDs.
    ///
    /// # Arguments
    ///
    /// * `jar_ids` - A `Vec<JarId>` containing the IDs of the deposit jars from which interest is being claimed.
    /// * `amount` - An optional `TokenAmount` specifying the desired amount of tokens to claim. If provided, the method
    ///              will attempt to claim this specific amount of tokens. If not provided or if the specified amount
    ///              is greater than the total available interest in the provided jars, the method will claim the maximum
    ///              available amount.
    /// * `detailed` – An optional boolean value specifying if the method must return only total amount of claimed tokens
    ///                or detailed summary for each claimed jar. Set it `true` to get a detailed result. In case of `false`
    ///                or `None` it returns only the total claimed amount.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<ClaimedAmountView>` representing the total amount of tokens claimed
    /// and probably a map containing amount of tokens claimed from each Jar.
    /// If the total available interest across the specified jars is zero or the provided `amount`
    /// is zero, the total amount in returned object will also be zero and the detailed map will be empty (if requested).
    fn claim_jars(
        &mut self,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
        detailed: Option<bool>,
    ) -> PromiseOrValue<ClaimedAmountView>;
}

/// The `JarApi` trait defines methods for managing deposit jars and their associated data within the smart contract.
#[make_integration_version]
pub trait JarApi {
    /// Retrieves information about a specific deposit jar by its index.
    ///
    /// # Arguments
    ///
    /// * `jar_id` - The ID of the deposit jar for which information is being retrieved.
    ///
    /// # Returns
    ///
    /// A `JarView` struct containing details about the specified deposit jar.
    fn get_jar(&self, account_id: AccountId, jar_id: JarIdView) -> JarView;

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
    /// * `jar_ids` - A `Vec<JarIdView>` containing the IDs of the deposit jars for which the
    ///                   principal is being retrieved.
    ///
    /// * `account_id` - The `AccountId` of the account for which the principal is being retrieved.
    ///
    /// # Returns
    ///
    /// An `U128` representing the sum of principal amounts for the specified deposit jars.
    fn get_principal(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> AggregatedTokenAmountView;

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
    /// * `jar_ids` - A `Vec<JarIdView>` containing the IDs of the deposit jars for which the
    ///                   interest is being retrieved.
    ///
    /// # Returns
    ///
    /// An `U128` representing the sum of interest amounts for the specified deposit jars.
    ///
    fn get_interest(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> AggregatedInterestView;

    /// Restakes the contents of a specified deposit jar into a new jar.
    ///
    /// # Arguments
    ///
    /// * `jar_id` - The ID of the deposit jar from which the restaking is being initiated.
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
    fn restake(&mut self, jar_id: JarIdView) -> JarView;
}

#[make_integration_version]
pub trait MigrationApi {
    /// Migrates `CeFi Jars` to create `DeFi Jars`.
    ///
    /// This method receives a list of entities called `CeFiJar`, which represent token deposits
    /// from a 3rd party service, and creates corresponding `DeFi Jars` for them. In order to support
    /// the transition of deposit terms from the 3rd party to the contract, the `Product` with these
    /// terms must be registered beforehand.
    ///
    /// # Arguments
    ///
    /// - `jars`: A vector of `CeFiJar` entities representing token deposits from a 3rd party service.
    /// - `total_received`: The total amount of tokens received, ensuring that all tokens are distributed
    ///   correctly.
    ///
    /// # Panics
    ///
    /// This method can panic in following cases:
    ///
    /// 1. If a `Product` required to create a Jar is not registered. In such a case, the migration
    ///    process cannot proceed, and the method will panic.
    ///
    /// 2. If the total amount of tokens received is not equal to the sum of all `CeFiJar` entities.
    ///    This panic ensures that all deposits are properly migrated, and any discrepancies will trigger
    ///    an error.
    ///
    /// 3. Panics in case of unauthorized access by non-admin users.
    ///
    /// # Authorization
    ///
    /// This method can only be called by the Contract Admin. Unauthorized access will result in a panic.
    ///
    fn migrate_jars(&mut self, jars: Vec<CeFiJar>, total_received: U128);
}

/// The `PenaltyApi` trait provides methods for applying or canceling penalties on premium jars within the smart contract.
#[make_integration_version]
pub trait PenaltyApi {
    /// Sets the penalty status for a specified jar.
    ///
    /// This method allows the contract manager to apply or cancel a penalty for a premium jar. Premium jars are those associated
    /// with products having Downgradable APY. When a user violates the terms of a premium product and a penalty is applied, the
    /// interest for the jar is calculated using a downgraded APY rate. If the terms are no longer violated, the penalty can be canceled.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account of user which owns this jar.
    /// * `jar_id` - The ID of the jar for which the penalty status is being modified.
    /// * `value` - A boolean value indicating whether the penalty should be applied (`true`) or canceled (`false`).
    ///
    /// # Panics
    ///
    /// This method will panic if the jar's associated product has a constant APY rather than a downgradable APY.
    fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool);

    /// Batched version of `set_penalty`
    ///
    /// # Arguments
    ///
    /// * `jars` - List of Account IDs and their Jar IDs to which penalty must be applied.
    /// * `value` - A boolean value indicating whether the penalty should be applied (`true`) or canceled (`false`).
    ///
    /// # Panics
    ///
    /// This method will panic if the jar's associated product has a constant APY rather than a downgradable APY.
    fn batch_set_penalty(&mut self, jars: Vec<(AccountId, Vec<JarIdView>)>, value: bool);
}

/// The `ProductApi` trait defines methods for managing products within the smart contract.
#[make_integration_version]
pub trait ProductApi {
    #[deposit_one_yocto]
    /// Registers a new product in the contract. This function can only be called by the administrator.
    ///
    /// # Arguments
    ///
    /// * `command` - A `RegisterProductCommand` struct containing information about the new product.
    ///
    /// # Panics
    ///
    /// This method will panic if a product with the same id already exists.
    fn register_product(&mut self, command: RegisterProductCommand);

    #[deposit_one_yocto]
    /// Sets the enabled status of a specific product.
    ///
    /// This method allows modifying the enabled status of a product, which determines whether users can create
    /// jars for the specified product. If the `is_enabled` parameter is set to `true`, users will be able to create
    /// jars for the product. If set to `false`, any attempts to create jars for the product will be rejected.
    ///
    /// # Arguments
    ///
    /// * `product_id` - The ID of the product for which the enabled status is being modified.
    /// * `is_enabled` - A boolean value indicating whether the product should be enabled (`true`) or disabled (`false`).
    ///
    /// # Panics
    ///
    /// This method will panic if the provided `is_enabled` value matches the current enabled status of the product.
    fn set_enabled(&mut self, product_id: ProductId, is_enabled: bool);

    #[deposit_one_yocto]
    /// Sets a new public key for the specified product.
    ///
    /// This method allows replacing the existing public key associated with a product. This might be necessary
    /// in cases where a key pair is compromised, and an oracle needs to update the public key for security reasons.
    ///
    /// # Arguments
    ///
    /// * `product_id` - The ID of the product for which the public key is being replaced.
    /// * `public_key` - The new public key represented as a base64-encoded byte array.
    fn set_public_key(&mut self, product_id: ProductId, public_key: Base64VecU8);

    /// Retrieves a list of all registered products in the contract.
    ///
    /// # Returns
    ///
    /// A `Vec<ProductView>` containing information about all registered products.
    fn get_products(&self) -> Vec<ProductView>;
}

/// The `WithdrawApi` trait defines methods for withdrawing tokens from specific deposit jars within the smart contract.
#[make_integration_version]
pub trait WithdrawApi {
    /// Allows the owner of a deposit jar to withdraw a specified amount of tokens from it.
    ///
    /// # Arguments
    ///
    /// * `jar_id` - The ID of the deposit jar from which the withdrawal is being made.
    /// * `amount` - An optional `U128` value indicating the amount of tokens to withdraw. If `None` is provided,
    ///              the entire balance of the jar will be withdrawn.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<WithdrawView>` which represents the result of the withdrawal. If the withdrawal is successful,
    /// it returns the withdrawn amount and probably fee, if it's defined by the associated Product.
    /// If there are insufficient funds or other conditions are not met, the contract might panic or
    /// return 0 for both withdrawn amount and fee.
    ///
    /// # Panics
    ///
    /// This function may panic under the following conditions:
    /// - If the caller is not the owner of the specified jar.
    /// - If the withdrawal amount exceeds the available balance in the jar.
    /// - If attempting to withdraw from a Fixed jar that is not yet mature.
    fn withdraw(&mut self, jar_id: JarIdView, amount: Option<U128>) -> PromiseOrValue<WithdrawView>;
}

#[cfg(feature = "integration-methods")]
#[make_integration_version]
pub trait IntegrationTestMethods {
    fn block_timestamp_ms(&self) -> near_sdk::Timestamp;
    fn bulk_create_jars(
        &mut self,
        account_id: AccountId,
        product_id: ProductId,
        locked_amount: crate::TokenAmount,
        jars_count: u32,
    );
    fn total_jars_count(&self) -> usize;
}
