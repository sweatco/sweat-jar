use near_sdk::{json_types::U128, require};
use sweat_jar_model::{jar::CeFiJar, TokenAmount};

use crate::{
    event::{emit, EventKind, MigrationEventItem},
    jar::model::Jar,
    Contract,
};

impl Contract {
    pub(crate) fn migrate_jars(&mut self, jars: Vec<CeFiJar>, total_received: U128) {
        let mut event_data: Vec<MigrationEventItem> = vec![];
        let mut total_amount: TokenAmount = 0;

        for ce_fi_jar in jars {
            require!(
                self.products.contains_key(&ce_fi_jar.product_id),
                format!("Product {} is not registered", ce_fi_jar.product_id),
            );

            let id = self.increment_and_get_last_jar_id();

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
