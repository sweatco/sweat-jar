#![cfg(test)]

use base64::{engine::general_purpose, Engine};
use crypto_hash::{digest, Algorithm};
use ed25519_dalek::{Signer, SigningKey};
use general_purpose::STANDARD;
use near_sdk::AccountId;
use rand::rngs::OsRng;
use sweat_jar_model::{ProductId, TokenAmount, UDecimal, MS_IN_YEAR};

use crate::{
    common::tests::Context,
    jar::model::JarTicket,
    product::model::{
        v2::{Apy, Cap, DowngradableApy, FixedProductTerms, Terms, WithdrawalFee},
        ProductV2,
    },
    test_utils::PRODUCT,
    Contract,
};

pub(crate) struct MessageSigner {
    signing_key: SigningKey,
}

impl MessageSigner {
    pub(crate) fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key: SigningKey = SigningKey::generate(&mut csprng);

        Self { signing_key }
    }

    pub(crate) fn sign(&self, message: &str) -> Vec<u8> {
        let message_hash = digest(Algorithm::SHA256, message.as_bytes());
        let signature = self.signing_key.sign(message_hash.as_slice());
        signature.to_bytes().to_vec()
    }

    pub(crate) fn sign_base64(&self, message: &str) -> String {
        STANDARD.encode(self.sign(message))
    }

    pub(crate) fn public_key(&self) -> Vec<u8> {
        self.signing_key.verifying_key().as_ref().to_vec()
    }
}

impl ProductV2 {
    pub fn new() -> Self {
        Self {
            id: PRODUCT.to_string(),
            cap: Cap { min: 0, max: 1_000_000 },
            terms: Terms::Fixed(FixedProductTerms {
                lockup_term: MS_IN_YEAR,
                apy: Apy::new_downgradable(),
            }),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }

    pub fn with_id(mut self, id: ProductId) -> Self {
        self.id = id;
        self
    }

    pub fn with_terms(mut self, terms: Terms) -> Self {
        self.terms = terms;
        self
    }

    pub fn with_public_key(mut self, public_key: Option<Vec<u8>>) -> Self {
        self.public_key = public_key;
        self
    }
}

impl ProductV2 {
    pub(crate) fn id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub(crate) fn public_key(mut self, pk: Vec<u8>) -> Self {
        self.public_key = Some(pk);
        self
    }

    pub(crate) fn enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }

    pub(crate) fn cap(mut self, min: TokenAmount, max: TokenAmount) -> Self {
        self.cap = Cap { min, max };
        self
    }

    pub(crate) fn with_withdrawal_fee(mut self, fee: WithdrawalFee) -> Self {
        self.withdrawal_fee = Some(fee);
        self
    }

    pub(crate) fn terms(mut self, terms: Terms) -> Self {
        self.terms = terms;
        self
    }
}

impl Context {
    pub(crate) fn get_signature_material(
        &self,
        receiver_id: &AccountId,
        ticket: &JarTicket,
        amount: TokenAmount,
    ) -> String {
        Contract::get_signature_material(
            &self.owner,
            receiver_id,
            &ticket.product_id,
            amount,
            ticket.valid_until.0,
            0,
        )
    }
}

// TODO: move to tests
impl Apy {
    pub(crate) fn new_downgradable() -> Self {
        Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(20, 2),
            fallback: UDecimal::new(10, 2),
        })
    }
}
