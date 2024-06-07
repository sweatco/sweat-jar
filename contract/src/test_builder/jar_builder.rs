use near_sdk::NearToken;

use crate::jar::model::Jar;

pub(crate) trait JarBuilder: Sized {
    fn apply(self, jar: Jar) -> Jar;
    fn build(self, id: u32, product_id: &str, principal: u128) -> Jar {
        let jar = Jar::new(id)
            .product_id(product_id)
            .principal(NearToken::from_near(principal).as_yoctonear());
        self.apply(jar)
    }
}

impl JarBuilder for () {
    fn apply(self, jar: Jar) -> Jar {
        jar
    }
}
