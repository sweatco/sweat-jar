use near_sdk::{test_utils::test_env::alice, AccountId, NearToken};

use crate::jar::model::Jar;

pub(crate) enum JarField {
    Account(AccountId),
}

pub(crate) trait JarBuilder: Sized {
    fn apply(self, jar: Jar) -> Jar;
    fn build(self, id: u32, product_id: &str, principal: u128) -> Jar {
        let jar = Jar::new(id)
            .account_id(&alice())
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

impl JarBuilder for JarField {
    fn apply(self, jar: Jar) -> Jar {
        match self {
            JarField::Account(account_id) => jar.account_id(&account_id),
        }
    }
}

impl<const SIZE: usize> JarBuilder for [JarField; SIZE] {
    fn apply(self, jar: Jar) -> Jar {
        let mut jar = jar;
        for j in self {
            jar = j.apply(jar)
        }
        jar
    }
}
