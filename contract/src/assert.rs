use near_sdk::require;
use sweat_jar_model::TokenAmount;

use crate::jar::model::{Jar, JarV2};

pub(crate) fn assert_not_locked(jar: &JarV2) {
    require!(!jar.is_pending_withdraw, "Another operation on this Jar is in progress");
}
