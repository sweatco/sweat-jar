#![cfg(test)]

use base64::{engine::general_purpose, Engine};
use crypto_hash::{digest, Algorithm};
use ed25519_dalek::{Signer, SigningKey};
use fake::{Fake, Faker};
use general_purpose::STANDARD;
use near_sdk::AccountId;
use rand::rngs::OsRng;

use crate::{
    common::{tests::Context, udecimal::UDecimal, Duration, TokenAmount},
    jar::model::JarTicket,
    product::{
        model::{Apy, Cap, FixedProductTerms, Product, Terms, WithdrawalFee},
        tests::MS_IN_YEAR,
    },
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

impl Product {
    pub(crate) fn generate(id: &str) -> Self {
        Self {
            id: id.to_string(),
            apy: Apy::Constant(UDecimal::new((1..20).fake(), (1..2).fake())),
            cap: Cap {
                min: (0..1_000).fake(),
                max: (1_000_000..1_000_000_000).fake(),
            },
            terms: Terms::Fixed(FixedProductTerms {
                lockup_term: (1..3).fake::<u64>() * 31_536_000_000,
                allows_top_up: Faker.fake(),
                allows_restaking: Faker.fake(),
            }),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
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

    pub(crate) fn flexible(mut self) -> Self {
        self.terms = Terms::Flexible;
        self
    }

    pub(crate) fn with_withdrawal_fee(mut self, fee: WithdrawalFee) -> Self {
        self.withdrawal_fee = Some(fee);
        self
    }

    pub(crate) fn lockup_term(mut self, term: Duration) -> Self {
        self.terms = match self.terms {
            Terms::Fixed(terms) => Terms::Fixed(FixedProductTerms {
                lockup_term: term,
                ..terms
            }),
            Terms::Flexible => Terms::Fixed(FixedProductTerms {
                lockup_term: term,
                allows_top_up: false,
                allows_restaking: false,
            }),
        };

        self
    }

    pub(crate) fn with_allows_top_up(mut self, allows_top_up: bool) -> Self {
        self.terms = match self.terms {
            Terms::Fixed(terms) => Terms::Fixed(FixedProductTerms { allows_top_up, ..terms }),
            Terms::Flexible => Terms::Fixed(FixedProductTerms {
                allows_top_up,
                lockup_term: MS_IN_YEAR,
                allows_restaking: false,
            }),
        };

        self
    }

    pub(crate) fn with_allows_restaking(mut self, allows_restaking: bool) -> Self {
        self.terms = match self.terms {
            Terms::Fixed(terms) => Terms::Fixed(FixedProductTerms {
                allows_restaking,
                ..terms
            }),
            Terms::Flexible => Terms::Fixed(FixedProductTerms {
                allows_restaking,
                lockup_term: MS_IN_YEAR,
                allows_top_up: false,
            }),
        };

        self
    }

    pub(crate) fn apy(mut self, apy: Apy) -> Self {
        self.apy = apy;
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
            None,
        )
    }
}
