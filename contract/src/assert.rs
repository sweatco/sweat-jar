use near_sdk::require;

use crate::jar::model::JarV2;

pub(crate) fn assert_not_locked(jar: &JarV2) {
    require!(!jar.is_pending_withdraw, "Another operation on this Jar is in progress");
}
