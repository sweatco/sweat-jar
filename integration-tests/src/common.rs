use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;

pub(crate) fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key: VerifyingKey = VerifyingKey::from(&signing_key);

    (signing_key, verifying_key)
}
