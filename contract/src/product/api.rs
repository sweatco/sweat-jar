use near_sdk::{assert_one_yocto, env::panic_str, near_bindgen, require};
use sweat_jar_model::{
    api::ProductApi,
    product::{ProductView, RegisterProductCommand},
    ProductId,
};

use crate::{
    event::{emit, ChangeProductPublicKeyData, EnableProductData, EventKind},
    product::model::{Apy, Product},
    Base64VecU8, Contract, ContractExt,
};

#[near_bindgen]
impl ProductApi for Contract {
    #[payable]
    fn register_product(&mut self, command: RegisterProductCommand) {
        self.register_product_internal(command, false);
    }

    fn update_product(&mut self, command: RegisterProductCommand) {
        self.register_product_internal(command, true);
    }

    #[payable]
    fn set_enabled(&mut self, product_id: ProductId, is_enabled: bool) {
        self.assert_manager();
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
    fn set_public_key(&mut self, product_id: ProductId, public_key: Base64VecU8) {
        self.assert_manager();
        assert_one_yocto();

        let mut product = self.get_product(&product_id);
        product.public_key = Some(public_key.0.clone());
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

impl Contract {
    fn register_product_internal(&mut self, command: RegisterProductCommand, update: bool) {
        use crate::product::model::Terms;

        self.assert_manager();
        assert_one_yocto();

        if update {
            assert!(
                self.products.get(&command.id).is_some(),
                "This product can't be updated because it doesn't exist"
            );
        } else {
            assert!(self.products.get(&command.id).is_none(), "Product already exists");
        }

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

        self.products.insert(&product.id, &product);

        if update {
            emit(EventKind::UpdateProduct(product));
        } else {
            emit(EventKind::RegisterProduct(product));
        }
    }
}
