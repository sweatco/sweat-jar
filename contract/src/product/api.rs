use near_sdk::near_bindgen;

use crate::*;
use crate::Contract;
use crate::event::{emit, EventKind};
use crate::product::command::RegisterProductCommand;
use crate::product::model::Product;
use crate::product::view::ProductView;

/// The `ProductApi` trait defines methods for managing products within the smart contract.
pub trait ProductApi {
    /// Registers a new product in the contract. This function can only be called by the administrator.
    ///
    /// # Arguments
    ///
    /// * `command` - A `RegisterProductCommand` struct containing information about the new product.
    fn register_product(&mut self, command: RegisterProductCommand);

    /// Retrieves a list of all registered products in the contract.
    ///
    /// # Returns
    ///
    /// A `Vec<ProductView>` containing information about all registered products.
    fn get_products(&self) -> Vec<ProductView>;
}

#[near_bindgen]
impl ProductApi for Contract {
    fn register_product(&mut self, command: RegisterProductCommand) {
        self.assert_admin();

        let product: Product = command.into();
        self.products.insert(product.clone().id, product.clone());

        emit(EventKind::RegisterProduct(product));
    }

    fn get_products(&self) -> Vec<ProductView> {
        self.products.values().map(|product| product.clone().into()).collect()
    }
}