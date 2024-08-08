use near_sdk::{test_utils::test_env::alice, AccountId, NearToken};
use sweat_jar_model::Timezone;

use crate::jar::model::Jar;

pub(crate) enum JarField {
    Account(AccountId),
    Timezone(Timezone),
}

pub(crate) trait JarBuilder: Sized {
    fn apply(&self, jar: Jar) -> Jar;
    fn timezone(&self) -> Option<Timezone>;
    fn build(&self, id: u32, product_id: &str, principal: u128) -> Jar {
        let jar = Jar::new(id)
            .account_id(&alice())
            .product_id(product_id)
            .principal(NearToken::from_near(principal).as_yoctonear());
        self.apply(jar)
    }
}

impl JarBuilder for () {
    fn apply(&self, jar: Jar) -> Jar {
        jar
    }

    fn timezone(&self) -> Option<Timezone> {
        None
    }
}

impl JarBuilder for JarField {
    fn apply(&self, jar: Jar) -> Jar {
        match self {
            JarField::Account(account_id) => jar.account_id(&account_id),
            JarField::Timezone(_) => jar,
        }
    }

    fn timezone(&self) -> Option<Timezone> {
        match self {
            JarField::Timezone(tz) => Some(*tz),
            _ => None,
        }
    }
}

impl<const SIZE: usize> JarBuilder for [JarField; SIZE] {
    fn apply(&self, jar: Jar) -> Jar {
        let mut jar = jar;
        for j in self {
            jar = j.apply(jar)
        }
        jar
    }

    fn timezone(&self) -> Option<Timezone> {
        for j in self {
            if let JarField::Timezone(tz) = j {
                return Some(*tz);
            }
        }

        None
    }
}
