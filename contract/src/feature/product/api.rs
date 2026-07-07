use std::clone::Clone;

use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{assert_one_yocto, near, require};
use sweat_jar_model::{
    api::ProductApi,
    data::product::{Product, ProductAssertions, ProductId, ProductModelApi},
};

use crate::{
    common::event::{emit, ChangeProductPublicKeyData, EnableProductData, EventKind},
    Base64VecU8, Contract, ContractExt, Roles,
};

#[near]
impl ProductApi for Contract {
    #[access_control_any(roles(Roles::ProductManager))]
    #[payable]
    fn register_product(&mut self, product: Product) {
        assert_one_yocto();
        assert!(self.products.get(&product.id).is_none(), "Product already exists");
        product.assert_score_based_product_is_protected();
        product.assert_fee_amount();
        product.assert_cap_order();

        self.products.insert(&product.id, &product);

        emit(EventKind::RegisterProduct(product));
    }

    #[access_control_any(roles(Roles::ProductManager))]
    #[payable]
    fn set_enabled(&mut self, product_id: ProductId, is_enabled: bool) {
        assert_one_yocto();

        let mut product = self.get_product(&product_id);

        require!(is_enabled != product.is_enabled, "Status matches");

        product.is_enabled = is_enabled;

        self.products.insert(&product_id, &product);

        emit(EventKind::EnableProduct(EnableProductData { product_id, is_enabled }));
    }

    #[access_control_any(roles(Roles::ProductManager))]
    #[payable]
    fn set_public_key(&mut self, product_id: ProductId, public_key: Base64VecU8) {
        assert_one_yocto();

        let mut product = self.get_product(&product_id);
        product.set_public_key(Some(public_key.clone()));
        self.products.insert(&product_id, &product);

        emit(EventKind::ChangeProductPublicKey(ChangeProductPublicKeyData {
            product_id,
            pk: public_key,
        }));
    }

    fn get_products(&self) -> Vec<Product> {
        self.products.values().collect()
    }
}
