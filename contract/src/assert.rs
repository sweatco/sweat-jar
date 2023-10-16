use model::TokenAmount;
use near_sdk::{require, AccountId};

use crate::{common::Timestamp, jar::model::Jar, product::model::Product};

pub(crate) fn assert_not_locked(jar: &Jar) {
    require!(!jar.is_pending_withdraw, "Another operation on this Jar is in progress");
}

pub(crate) fn assert_sufficient_balance(jar: &Jar, amount: TokenAmount) {
    require!(jar.principal >= amount, "Insufficient balance");
}

pub(crate) fn assert_ownership(jar: &Jar, account_id: &AccountId) {
    assert_eq!(&jar.account_id, account_id, "Account doesn't own this jar");
}

pub(crate) fn assert_is_liquidable(jar: &Jar, product: &Product, now: Timestamp) {
    require!(jar.is_liquidable(product, now), "The jar is not mature yet");
}
