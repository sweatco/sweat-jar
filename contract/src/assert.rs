use ed25519_dalek::{Signature, SIGNATURE_LENGTH};

use crate::jar::Jar;

pub(crate) fn assert_is_not_locked(jar: &Jar) {
    assert!(
        !jar.is_pending_withdraw,
        "Jar is locked. Probably some operation on it is in progress."
    );
}

pub(crate) fn assert_signature_is_valid(signature: &[u8]) {
    assert_eq!(
        signature.len(),
        SIGNATURE_LENGTH,
        "Invalid signature length"
    );
}
