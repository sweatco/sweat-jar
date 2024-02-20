use near_sdk::{json_types::U128, require};
use sweat_jar_model::{jar::CeFiJar, TokenAmount};

use crate::{
    event::{emit, EventKind, MigrationEventItem},
    jar::model::Jar,
    Contract,
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

        for ce_fi_jar in jars {
            require!(
                self.products.contains_key(&ce_fi_jar.product_id),
                format!("Product {} is not registered", ce_fi_jar.product_id),
            );

            let id = self.increment_and_get_last_jar_id();

            self.migrate_account_jars_if_needed(ce_fi_jar.account_id.clone());
            let account_jars = self.account_jars.entry(ce_fi_jar.account_id.clone()).or_default();

            let jar = Jar {
                id,
                account_id: ce_fi_jar.account_id,
                product_id: ce_fi_jar.product_id,
                created_at: ce_fi_jar.created_at.0,
                principal: ce_fi_jar.principal.0,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: 0,
            };

            total_amount += jar.principal;

            event_data.push(MigrationEventItem {
                original_id: ce_fi_jar.id,
                id: jar.id,
                account_id: jar.account_id.clone(),
            });

            account_jars.push(jar);
        }

        require!(
            total_received.0 == total_amount,
            "Total received doesn't match the sum of principals"
        );

        emit(EventKind::Migration(event_data));
    }
}
