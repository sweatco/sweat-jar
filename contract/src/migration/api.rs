use near_sdk::{__private::schemars::Set, json_types::U128, require};

use crate::{
    common::TokenAmount,
    event::{emit, EventKind, MigrationEventItem},
    migration::model::CeFiJar,
    product::model::ProductId,
    Contract, HashSet, Jar, JarState,
};

impl Contract {
    pub(crate) fn migrate_jars(&mut self, jars: Vec<CeFiJar>, total_received: U128) {
        let mut event_data: Vec<MigrationEventItem> = vec![];
        let mut total_amount: TokenAmount = 0;

        let product_ids: Set<ProductId> = self.products.keys().cloned().collect();

        for ce_fi_jar in jars {
            require!(
                product_ids.contains(&ce_fi_jar.product_id),
                format!("Product {} is not registered", ce_fi_jar.product_id),
            );

            let index = self.jars.len();

            let jar = Jar {
                index,
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

            self.jars.push(jar.clone());

            let mut account_jars = self
                .account_jars
                .get(&jar.account_id)
                .map_or_else(HashSet::new, Clone::clone);
            account_jars.insert(jar.index);
            self.account_jars.insert(jar.clone().account_id, account_jars);

            total_amount += jar.principal;

            event_data.push(MigrationEventItem {
                original_id: ce_fi_jar.id,
                index: jar.index,
                account_id: jar.account_id,
            });
        }

        assert_eq!(
            total_received.0, total_amount,
            "Total received doesn't match the sum of principals"
        );

        emit(EventKind::Migration(event_data));
    }
}
