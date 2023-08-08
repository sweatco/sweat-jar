use near_sdk::near_bindgen;
use crate::*;
use crate::Contract;
use crate::event::{emit, EventKind};
use crate::product::command::RegisterProductCommand;
use crate::product::model::Product;
use crate::product::view::ProductView;

pub trait ProductApi {
    fn register_product(&mut self, command: RegisterProductCommand);
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