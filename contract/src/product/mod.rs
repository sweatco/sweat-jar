pub mod api;
pub mod command;
pub mod model;
pub mod view;

#[cfg(test)]
pub(crate) mod tests {
    use near_sdk::{json_types::Base64VecU8, test_utils::accounts};

    use crate::{
        common::{tests::Context, Duration, UDecimal},
        product::{
            api::ProductApi,
            command::{RegisterProductCommand, TermsDto, WithdrawalFeeDto},
            model::{Apy, Cap, FixedProductTerms, Product, Terms},
        },
    };

    pub(crate) const YEAR_IN_MS: Duration = 365 * 24 * 60 * 60 * 1000;

    pub(crate) fn get_product() -> Product {
        Product {
            id: "product".to_string(),
            apy: Apy::Constant(UDecimal::new(12, 2)),
            cap: Cap {
                min: 100,
                max: 100_000_000_000,
            },
            terms: Terms::Fixed(FixedProductTerms {
                lockup_term: YEAR_IN_MS,
                allows_top_up: false,
                allows_restaking: false,
            }),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }

    pub(crate) fn get_product_with_fee_command(fee: WithdrawalFeeDto) -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product_with_fee".to_string(),
            withdrawal_fee: Some(fee),
            ..Default::default()
        }
    }

    pub(crate) fn get_register_product_command() -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product".to_string(),
            ..Default::default()
        }
    }

    pub(crate) fn get_register_flexible_product_command() -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product_flexible".to_string(),
            terms: TermsDto::Flexible,
            ..Default::default()
        }
    }

    #[test]
    fn disable_product_when_enabled() {
        let admin = accounts(0);
        let reference_product = &Product::generate("product").enabled(true);

        let mut context = Context::new(admin.clone()).with_products(&[reference_product.clone()]);

        let mut product = context.contract.get_product(&reference_product.id);
        assert!(product.is_enabled);

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.set_enabled(reference_product.id.to_string(), false)
        });

        product = context.contract.get_product(&reference_product.id);
        assert!(!product.is_enabled);
    }

    #[test]
    #[should_panic(expected = "Status matches")]
    fn enable_product_when_enabled() {
        let admin = accounts(0);
        let reference_product = &Product::generate("product").enabled(true);

        let mut context = Context::new(admin.clone()).with_products(&[reference_product.clone()]);

        let product = context.contract.get_product(&reference_product.id);
        assert!(product.is_enabled);

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.set_enabled(reference_product.id.to_string(), true)
        });
    }

    #[test]
    #[should_panic(expected = "Product already exists")]
    fn register_product_with_existing_id() {
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        context.with_deposit_yocto(1, |context| {
            let first_command = get_register_product_command();
            context.contract.register_product(first_command)
        });

        context.with_deposit_yocto(1, |context| {
            let second_command = get_register_product_command();
            context.contract.register_product(second_command)
        });
    }

    #[test]
    fn set_public_key() {
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .set_public_key(get_register_product_command().id, Base64VecU8(vec![0, 1, 2]))
        });

        let product = context
            .contract
            .products
            .get(&get_register_product_command().id)
            .unwrap();
        assert_eq!(vec![0, 1, 2], product.clone().public_key.unwrap());
    }

    #[test]
    #[should_panic(expected = "Can be performed only by admin")]
    fn set_public_key_by_not_admin() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context.switch_account(&alice);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .set_public_key(get_register_product_command().id, Base64VecU8(vec![0, 1, 2]))
        });
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn set_public_key_without_deposit() {
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        context
            .contract
            .set_public_key(get_register_product_command().id, Base64VecU8(vec![0, 1, 2]));
    }

    #[test]
    fn assert_cap_in_bounds() {
        get_product().assert_cap(200);
    }

    #[test]
    #[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
    fn assert_cap_less_than_min() {
        get_product().assert_cap(10);
    }

    #[test]
    #[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
    fn assert_cap_more_than_max() {
        get_product().assert_cap(500_000_000_000);
    }
}

#[cfg(test)]
pub(crate) mod helpers {
    use base64::{engine::general_purpose, Engine};
    use crypto_hash::{digest, Algorithm};
    use ed25519_dalek::{Keypair, Signer};
    use fake::{Fake, Faker};
    use general_purpose::STANDARD;
    use near_sdk::AccountId;
    use rand::rngs::OsRng;

    use crate::{
        common::{tests::Context, Duration, TokenAmount, UDecimal},
        jar::model::JarTicket,
        product::{
            model::{Apy, Cap, FixedProductTerms, Product, Terms},
            tests::YEAR_IN_MS,
        },
        Contract,
    };

    pub(crate) struct MessageSigner {
        keypair: Keypair,
    }

    impl MessageSigner {
        pub(crate) fn new() -> Self {
            let mut csprng = OsRng {};
            let keypair = Keypair::generate(&mut csprng);

            Self { keypair }
        }

        pub(crate) fn sign(&self, message: &str) -> Vec<u8> {
            let message_hash = digest(Algorithm::SHA256, message.as_bytes());
            let signature = self.keypair.sign(message_hash.as_slice());
            signature.to_bytes().to_vec()
        }

        pub(crate) fn sign_base64(&self, message: &str) -> String {
            STANDARD.encode(self.sign(message))
        }

        pub(crate) fn public_key(&self) -> &[u8; 32] {
            self.keypair.public.as_bytes()
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
                    lockup_term: YEAR_IN_MS,
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
                    lockup_term: YEAR_IN_MS,
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
}
