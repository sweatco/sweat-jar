use near_sdk::{assert_one_yocto, near_bindgen, require};

use crate::*;
use crate::Contract;
use crate::event::{emit, EnableProductData, EventKind};
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

    /// Retrieves a list of all registered products in the contract.
    ///
    /// # Returns
    ///
    /// A `Vec<ProductView>` containing information about all registered products.
    fn get_products(&self) -> Vec<ProductView>;
}

#[near_bindgen]
impl ProductApi for Contract {
    #[payable]
    fn register_product(&mut self, command: RegisterProductCommand) {
        self.assert_manager();
        assert_one_yocto();

        let product: Product = command.into();
        self.products.insert(product.clone().id, product.clone());

        emit(EventKind::RegisterProduct(product));
    }

    fn set_enabled(&mut self, product_id: ProductId, is_enabled: bool) {
        self.assert_manager();
        assert_one_yocto();

        let product = self.get_product(&product_id);

        require!(is_enabled != product.is_enabled, "Status matches");

        let id = &product.id;
        let updated_product = Product {
            is_enabled,
            ..product.clone()
        };
        self.products.insert(id.clone(), updated_product);

        emit(EventKind::EnableProduct(EnableProductData { id: product_id, is_enabled }));
    }

    fn get_products(&self) -> Vec<ProductView> {
        self.products.values().map(|product| product.clone().into()).collect()
    }
}