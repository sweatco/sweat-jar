use sweat_jar_model::Steps;

use crate::product::model::Product;

pub(crate) trait ProductBuilder: Sized {
    fn apply(self, product: Product) -> Product;
    fn build(self, id: &'static str) -> Product {
        let product = Product::new().id(id);
        self.apply(product)
    }
}

pub(crate) enum ProductField {
    APY(u32),
    StepsCap(Steps),
    NoStepsCap,
}

impl ProductBuilder for ProductField {
    fn apply(self, product: Product) -> Product {
        match self {
            ProductField::APY(apy) => product.apy(apy),
            ProductField::StepsCap(cap) => product.steps_cap(cap),
            ProductField::NoStepsCap => product.steps_cap(Steps::MAX),
        }
    }
}

impl<const SIZE: usize> ProductBuilder for [ProductField; SIZE] {
    fn apply(self, product: Product) -> Product {
        let mut product = product;
        for p in self {
            product = p.apply(product)
        }
        product
    }
}
