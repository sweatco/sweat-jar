use std::clone::Clone;

use near_sdk::{assert_one_yocto, near_bindgen, require};
use sweat_jar_model::{api::ProductApi, product::Product, ProductId};

use crate::{
    event::{emit, ChangeProductPublicKeyData, EnableProductData, EventKind},
    product::model::v1::{ProductAssertions, ProductModelApi},
    Base64VecU8, Contract, ContractExt,
};

#[near_bindgen]
impl ProductApi for Contract {
    #[payable]
    fn register_product(&mut self, product: Product) {
        self.assert_manager();
        assert_one_yocto();
        assert!(self.products.get(&product.id).is_none(), "Product already exists");
        product.assert_score_based_product_is_protected();
        product.assert_fee_amount();

        self.products.insert(&product.id, &product);

        emit(EventKind::RegisterProduct(product));
    }

    #[payable]
    fn set_enabled(&mut self, product_id: ProductId, is_enabled: bool) {
        self.assert_manager();
        assert_one_yocto();

        let mut product = self.get_product(&product_id);

        require!(is_enabled != product.is_enabled, "Status matches");

        product.is_enabled = is_enabled;

        self.products.insert(&product_id, &product);

        emit(EventKind::EnableProduct(EnableProductData { product_id, is_enabled }));
    }

    #[payable]
    fn set_public_key(&mut self, product_id: ProductId, public_key: Base64VecU8) {
        self.assert_manager();
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
