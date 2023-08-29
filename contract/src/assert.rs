use near_sdk::{require, AccountId};

use crate::{
    common::{Timestamp, TokenAmount},
    jar::model::{Jar, JarState},
    product::model::Product,
};

pub(crate) fn assert_is_not_closed(jar: &Jar) {
    assert_ne!(jar.state, JarState::Closed, "Jar is closed");
}

pub(crate) fn assert_sufficient_balance(jar: &Jar, amount: TokenAmount) {
    require!(jar.principal >= amount, "Insufficient balance");
}

pub(crate) fn assert_ownership(jar: &Jar, account_id: &AccountId) {
    assert_eq!(jar.account_id, account_id.clone(), "Account doesn't own this jar");
}

pub(crate) fn assert_is_liquidable(jar: &Jar, product: &Product, now: Timestamp) {
    require!(jar.is_liquidable(product, now), "The jar is not mature yet");
}
