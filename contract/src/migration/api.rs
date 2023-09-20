use near_sdk::{__private::schemars::Set, json_types::U128, require};

use crate::{
    common::TokenAmount,
    event::{emit, EventKind, MigrationEventItem},
    migration::model::CeFiJar,
    product::model::ProductId,
    Contract, Jar, JarState,
};

impl Contract {
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
    pub(crate) fn migrate_jars(&mut self, jars: Vec<CeFiJar>, total_received: U128) {
        let mut event_data: Vec<MigrationEventItem> = vec![];
        let mut total_amount: TokenAmount = 0;

        let product_ids: Set<ProductId> = self.products.keys().cloned().collect();

        for ce_fi_jar in jars {
            require!(
                product_ids.contains(&ce_fi_jar.product_id),
                format!("Product {} is not registered", ce_fi_jar.product_id),
            );

            // let index = self.jars.len();

            let index = 0;

            todo!();

            let jar = Jar {
                id: index,
                account_id: ce_fi_jar.account_id,
                product_id: ce_fi_jar.product_id,
                created_at: ce_fi_jar.created_at.0,
                principal: ce_fi_jar.principal.0,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                state: JarState::Active,
                is_penalty_applied: false,
            };

            // self.jars.push(jar.clone());
            todo!();

            self.account_jars
                .entry(jar.account_id.clone())
                .or_default()
                .jars
                .insert(jar.clone());

            total_amount += jar.principal;

            event_data.push(MigrationEventItem {
                original_id: ce_fi_jar.id,
                index: jar.id,
                account_id: jar.account_id,
            });
        }

        require!(
            total_received.0 == total_amount,
            "Total received doesn't match the sum of principals"
        );

        emit(EventKind::Migration(event_data));
    }
}
