use near_sdk::AccountId;

use crate::jar::{Jar, JarState};

pub(crate) fn assert_is_not_empty(jar: &Jar) {
    assert!(jar.principal > 0, "Jar is empty");
}

pub(crate) fn assert_is_not_closed(jar: &Jar) {
    assert!(jar.state != JarState::Closed, "Jar is closed");
}

pub(crate) fn assert_ownership(jar: &Jar, account_id: &AccountId) {
    assert_eq!(
        jar.account_id,
        account_id.clone(),
        "Account doesn't own this jar"
    );
}
