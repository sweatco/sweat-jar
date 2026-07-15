use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{assert_one_yocto, env::panic_str, near, require};
use sweat_jar_model::{
    api::ProductApi,
    product::{ProductView, RegisterProductCommand},
    ProductId,
};

use crate::{
    event::{emit, ChangeProductPublicKeyData, EnableProductData, EventKind},
    product::model::{Apy, Product, Terms},
    Base64VecU8, Contract, ContractExt, Roles,
};

#[near]
impl ProductApi for Contract {
    #[payable]
    #[access_control_any(roles(Roles::ProductManager))]
    fn register_product(&mut self, command: RegisterProductCommand) {
        assert_one_yocto();

        assert!(self.products.get(&command.id).is_none(), "Product already exists");

        let product: Product = command.into();

        if product.is_score_product() {
            let apy = match product.apy {
                Apy::Constant(apy) => apy,
                Apy::Downgradable(_) => panic_str("Step based products do not support downgradable APY"),
            };

            assert!(apy.is_zero(), "Step based products do not support constant APY");

            if let Terms::Fixed(fixed) = &product.terms {
                assert!(!fixed.allows_top_up, "Step based products don't support top up");
            }
        }

        product.assert_fee_amount();
        product.assert_public_key_valid();

        self.products.insert(&product.id, &product);

        emit(EventKind::RegisterProduct(product));
    }

    #[payable]
    #[access_control_any(roles(Roles::ProductManager))]
    fn set_enabled(&mut self, product_id: ProductId, is_enabled: bool) {
        assert_one_yocto();

        let mut product = self.get_product(&product_id);

        require!(is_enabled != product.is_enabled, "Status matches");

        product.is_enabled = is_enabled;

        self.products.insert(&product_id, &product);

        emit(EventKind::EnableProduct(EnableProductData {
            id: product_id,
            is_enabled,
        }));
    }

    #[payable]
    #[access_control_any(roles(Roles::ProductManager))]
    fn set_public_key(&mut self, product_id: ProductId, public_key: Base64VecU8) {
        assert_one_yocto();

        let mut product = self.get_product(&product_id);
        product.public_key = Some(public_key.0.clone());
        product.assert_public_key_valid();
        self.products.insert(&product_id, &product);

        emit(EventKind::ChangeProductPublicKey(ChangeProductPublicKeyData {
            product_id,
            pk: public_key,
        }));
    }

    fn get_products(&self) -> Vec<ProductView> {
        self.products.values().map(|product| product.clone().into()).collect()
    }
}
