pub mod api;
pub mod command;
pub mod model;
pub mod view;

#[cfg(test)]
pub(crate) mod tests {
    use near_sdk::{
        json_types::{Base64VecU8, U128, U64},
        test_utils::accounts,
    };

    use crate::{
        common::{tests::Context, UDecimal},
        product::{
            api::ProductApi,
            command::{FixedProductTermsDto, RegisterProductCommand, TermsDto, WithdrawalFeeDto},
            model::{Apy, Cap, FixedProductTerms, Product, Terms},
        },
    };

    fn get_premium_product_public_key() -> Vec<u8> {
        vec![
            33, 80, 163, 149, 64, 30, 150, 45, 68, 212, 97, 122, 213, 118, 189, 174, 239, 109, 48, 82, 50, 35, 197,
            176, 50, 211, 183, 128, 207, 1, 8, 68,
        ]
    }

    pub(crate) fn get_product() -> Product {
        Product {
            id: "product".to_string(),
            apy: Apy::Constant(UDecimal::new(12, 2)),
            cap: Cap {
                min: 100,
                max: 100_000_000_000,
            },
            terms: Terms::Fixed(FixedProductTerms {
                lockup_term: 365 * 24 * 60 * 60 * 1000,
                allows_top_up: false,
                allows_restaking: false,
            }),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }

    pub(crate) fn get_fee_product_command(fee: WithdrawalFeeDto) -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product_with_fee".to_string(),
            apy_default: (U128(12), 2),
            apy_fallback: None,
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            terms: TermsDto::Fixed(FixedProductTermsDto {
                lockup_term: U64(365 * 24 * 60 * 60 * 1000),
                allows_restaking: false,
                allows_top_up: false,
            }),
            withdrawal_fee: Some(fee),
            public_key: None,
            is_enabled: true,
        }
    }

    pub(crate) fn get_register_product_command() -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product".to_string(),
            apy_default: (U128(12), 2),
            apy_fallback: None,
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            terms: TermsDto::Fixed(FixedProductTermsDto {
                lockup_term: U64(365 * 24 * 60 * 60 * 1000),
                allows_restaking: false,
                allows_top_up: false,
            }),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }

    pub(crate) fn get_register_flexible_product_command() -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product_flexible".to_string(),
            apy_default: (U128(12), 2),
            apy_fallback: None,
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            terms: TermsDto::Flexible,
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }

    pub(crate) fn get_register_restakable_product_command() -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product_restakable".to_string(),
            apy_default: (U128(12), 2),
            apy_fallback: None,
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            terms: TermsDto::Fixed(FixedProductTermsDto {
                lockup_term: U64(365 * 24 * 60 * 60 * 1000),
                allows_restaking: true,
                allows_top_up: false,
            }),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }

    pub(crate) fn get_register_premium_product_command(public_key: Option<Base64VecU8>) -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product_premium".to_string(),
            apy_default: (U128(20), 2),
            apy_fallback: Some((U128(10), 2)),
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            terms: TermsDto::Fixed(FixedProductTermsDto {
                lockup_term: U64(365 * 24 * 60 * 60 * 1000),
                allows_top_up: false,
                allows_restaking: false,
            }),
            withdrawal_fee: None,
            public_key: public_key.or_else(|| Some(Base64VecU8(get_premium_product_public_key()))),
            is_enabled: true,
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
    use rand::rngs::OsRng;

    use crate::{
        common::{Duration, TokenAmount, UDecimal},
        product::model::{Apy, Cap, FixedProductTerms, Product, Terms},
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

        pub(crate) fn sign(&self, message: &str) -> String {
            let message_hash = digest(Algorithm::SHA256, message.as_bytes());
            let signature = self.keypair.sign(message_hash.as_slice());
            let signature_bytes = signature.to_bytes().to_vec();

            STANDARD.encode(signature_bytes)
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
                    lockup_term: 365 * 24 * 60 * 60 * 1_000,
                    allows_restaking: false,
                }),
            };

            self
        }

        pub(crate) fn apy(mut self, apy: Apy) -> Self {
            self.apy = apy;
            self
        }
    }
}
