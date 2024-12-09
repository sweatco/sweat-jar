use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use sweat_jar_model::{jar::JarView, TokenAmount};

pub(crate) fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key: VerifyingKey = VerifyingKey::from(&signing_key);

    (signing_key, verifying_key)
}

pub(crate) fn total_principal(jars: &Vec<JarView>) -> TokenAmount {
    jars.iter().map(|jar| jar.principal.0).sum()
}
