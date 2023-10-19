use model::{jar::JarIdView, ProductId};
use near_sdk::require;
use std::collections::HashMap;
use std::ptr::read;

use crate::{env, jar::model::JarId, AccountId, Contract, Jar, Product};

impl Contract {
    pub(crate) fn assert_manager(&self) {
        require!(
            self.manager == env::predecessor_account_id(),
            "Can be performed only by admin"
        );
    }

    pub(crate) fn assert_from_ft_contract(&self) {
        require!(
            env::predecessor_account_id() == self.token_account_id,
            format!("Can receive tokens only from {}", self.token_account_id)
        );
    }

    pub(crate) fn increment_and_get_last_jar_id(&mut self) -> JarId {
        self.last_jar_id += 1;
        self.last_jar_id
    }

    pub(crate) fn get_product(&self, product_id: &ProductId) -> &Product {
        self.products
            .get(product_id)
            .unwrap_or_else(|| env::panic_str(&format!("Product '{product_id}' doesn't exist")))
    }

    pub(crate) fn get_product_mut(&mut self, product_id: &ProductId) -> &mut Product {
        self.products
            .get_mut(product_id)
            .unwrap_or_else(|| env::panic_str(&format!("Product '{product_id}' doesn't exist")))
    }

    pub(crate) fn account_jars(&self, account_id: &AccountId) -> &[Jar] {
        self.account_jars.get(account_id).map_or(&[], |jars| jars.as_slice())
    }

    pub(crate) fn account_jars_with_ids(&self, account_id: &AccountId, ids: &[JarIdView]) -> Vec<&Jar> {
        // iterates once over jars and once over ids
        let jars: HashMap<JarId, &Jar> = self.account_jars(account_id).iter().map(|jar| (jar.id, jar)).collect();

        ids.iter()
            .map(|id| {
                *jars
                    .get(&id.0)
                    .unwrap_or_else(|| env::panic_str(&format!("Jar with id: '{}' doesn't exist", id.0)))
            })
            .collect()
    }

    pub(crate) fn account_jars_with_ids_mut(
        &mut self,
        account_id: &AccountId,
        ids: &[JarIdView],
    ) -> Vec<(&Product, &mut Jar)> {
        let Some(jars) = self
            .account_jars
            .get_mut(account_id)
            .map(|jars| jars.jars.as_mut_slice()) else {
            return vec![];
        } ;

        // iterates once over jars and once over ids
        let jars: HashMap<JarId, (&Product, &mut Jar)> = jars
            .iter_mut()
            .map(|jar| {
                (
                    jar.id,
                    (
                        self.products
                            .get(&jar.product_id)
                            .unwrap_or_else(|| env::panic_str(&format!("Product '{}' doesn't exist", jar.product_id))),
                        jar,
                    ),
                )
            })
            .collect();

        ids.iter()
            .map(|id| {
                let reference = jars
                    .get(&id.0)
                    .unwrap_or_else(|| env::panic_str(&format!("Jar with id: '{}' doesn't exist", id.0)))
                    as *const _;

                unsafe { read(reference) }
            })
            .collect()
    }

    pub(crate) fn add_new_jar(&mut self, account_id: &AccountId, jar: Jar) {
        let jars = self.account_jars.entry(account_id.clone()).or_default();
        jars.last_id = jar.id;
        jars.push(jar);
    }
}
