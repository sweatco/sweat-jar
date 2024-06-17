use sweat_jar_model::Score;

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
    ScoreCap(Score),
    NoScoreCap,
}

impl ProductBuilder for ProductField {
    fn apply(self, product: Product) -> Product {
        match self {
            ProductField::APY(apy) => product.apy(apy),
            ProductField::ScoreCap(cap) => product.score_cap(cap),
            ProductField::NoScoreCap => product.score_cap(Score::MAX),
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
