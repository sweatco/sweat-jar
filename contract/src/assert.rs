use crate::jar::Jar;

pub(crate) fn assert_is_not_locked(jar: &Jar) {
    assert!(!jar.is_locked, "Jar is locked. Probably some operation on it is in progress.");
}