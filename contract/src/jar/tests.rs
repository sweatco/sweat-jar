#![cfg(test)]

use fake::Fake;
use near_sdk::{test_utils::accounts, Timestamp};
use sweat_jar_model::MS_IN_YEAR;

use crate::{
    common::udecimal::UDecimal,
    product::model::{Apy, Product},
    Jar,
};

#[test]
fn get_interest_before_maturity() {
    let product = Product::generate("product")
        .apy(Apy::Constant(UDecimal::new(12, 2)))
        .lockup_term(2 * MS_IN_YEAR);
    let jar = Jar::generate(0, &accounts(0), &product.id).principal(100_000_000);

    let interest = jar.get_interest(&product, MS_IN_YEAR).0;
    assert_eq!(12_000_000, interest);
}

#[test]
fn get_interest_after_maturity() {
    let product = Product::generate("product")
        .apy(Apy::Constant(UDecimal::new(12, 2)))
        .lockup_term(MS_IN_YEAR);
    let jar = Jar::generate(0, &accounts(0), &product.id).principal(100_000_000);

    let interest = jar.get_interest(&product, 400 * 24 * 60 * 60 * 1000).0;
    assert_eq!(12_000_000, interest);
}

#[test]
fn interest_precision() {
    let product = Product::generate("product")
        .apy(Apy::Constant(UDecimal::new(1, 0)))
        .lockup_term(MS_IN_YEAR);
    let jar = Jar::generate(0, &accounts(0), &product.id).principal(MS_IN_YEAR as u128);

    assert_eq!(jar.get_interest(&product, 10000000000).0, 10000000000);
    assert_eq!(jar.get_interest(&product, 10000000001).0, 10000000001);

    for _ in 0..100 {
        let time: Timestamp = (10..MS_IN_YEAR).fake();
        assert_eq!(jar.get_interest(&product, time).0, time as u128);
    }
}

#[cfg(test)]
mod signature_tests {

    use near_sdk::{
        json_types::{Base64VecU8, U128, U64},
        test_utils::{
            accounts,
            test_env::{alice, bob, carol},
        },
    };
    use sweat_jar_model::{
        api::{JarApi, ProductApi},
        MS_IN_YEAR, U32,
    };

    use crate::{
        common::{tests::Context, udecimal::UDecimal},
        jar::model::JarTicket,
        product::{
            helpers::MessageSigner,
            model::{Apy, DowngradableApy, Product},
        },
        test_utils::{admin, expect_panic},
        Jar,
    };

    #[test]
    fn verify_ticket_with_valid_signature_and_date() {
        let admin = accounts(0);

        let signer = MessageSigner::new();
        let reference_product = generate_premium_product("premium_product", &signer);
        let context = Context::new(admin.clone()).with_products(&[reference_product.clone()]);

        let amount = 14_000_000;
        let ticket = JarTicket {
            product_id: reference_product.id,
            valid_until: U64(123000000),
        };

        let signature = signer.sign(context.get_signature_material(&admin, &ticket, amount).as_str());

        context
            .contract()
            .verify(&admin, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Signature must be 64 bytes")]
    fn verify_ticket_with_invalid_signature() {
        let alice = accounts(0);
        let admin = accounts(1);

        let signer = MessageSigner::new();
        let reference_product = generate_premium_product("premium_product", &signer);
        let context = Context::new(admin).with_products(&[reference_product.clone()]);

        let amount = 1_000_000;
        let ticket = JarTicket {
            product_id: reference_product.id,
            valid_until: U64(100000000),
        };

        let signature: Vec<u8> = vec![0, 1, 2];

        context
            .contract()
            .verify(&alice, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Not matching signature")]
    fn verify_ticket_with_not_matching_signature() {
        let admin = accounts(0);

        let signer = MessageSigner::new();
        let product = generate_premium_product("premium_product", &signer);
        let another_product = generate_premium_product("another_premium_product", &MessageSigner::new());

        let context = Context::new(admin.clone()).with_products(&[product, another_product.clone()]);

        let amount = 15_000_000;
        let ticket_for_another_product = JarTicket {
            product_id: another_product.id,
            valid_until: U64(100000000),
        };

        // signature made for wrong product
        let signature = signer.sign(
            context
                .get_signature_material(&admin, &ticket_for_another_product, amount)
                .as_str(),
        );

        context.contract().verify(
            &admin,
            amount,
            &ticket_for_another_product,
            Some(Base64VecU8(signature)),
        );
    }

    #[test]
    #[should_panic(expected = "Ticket is outdated")]
    fn verify_ticket_with_invalid_date() {
        let alice = accounts(0);
        let admin = accounts(1);

        let signer = MessageSigner::new();
        let reference_product = generate_premium_product("premium_product", &signer);
        let mut context = Context::new(admin).with_products(&[reference_product.clone()]);

        context.set_block_timestamp_in_days(365);

        let amount = 5_000_000;
        let ticket = JarTicket {
            product_id: reference_product.id,
            valid_until: U64(100000000),
        };

        let signature = signer.sign(context.get_signature_material(&alice, &ticket, amount).as_str());

        context
            .contract()
            .verify(&alice, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Product 'not_existing_product' doesn't exist")]
    fn verify_ticket_with_not_existing_product() {
        let admin = accounts(0);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        let signer = MessageSigner::new();
        let not_existing_product = generate_premium_product("not_existing_product", &signer);

        let amount = 500_000;
        let ticket = JarTicket {
            product_id: not_existing_product.id,
            valid_until: U64(100000000),
        };

        let signature = signer.sign(context.get_signature_material(&admin, &ticket, amount).as_str());

        context
            .contract()
            .verify(&admin, amount, &ticket, Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Signature is required")]
    fn verify_ticket_without_signature_when_required() {
        let admin = accounts(0);

        let signer = MessageSigner::new();
        let product = generate_premium_product("not_existing_product", &signer);
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 3_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(100000000),
        };

        context.contract().verify(&admin, amount, &ticket, None);
    }

    #[test]
    fn verify_ticket_without_signature_when_not_required() {
        let admin = accounts(0);

        let product = generate_product("regular_product");
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 4_000_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };

        context.contract().verify(&admin, amount, &ticket, None);
    }

    #[test]
    fn restake_by_not_owner() {
        let alice = alice();

        let product = generate_product("restakable_product").with_allows_restaking(true);
        let alice_jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut ctx = Context::new(admin())
            .with_products(&[product])
            .with_jars(&[alice_jar.clone()]);

        ctx.switch_account(bob());
        expect_panic(&ctx, "Account 'bob.near' doesn't exist", |ctx| {
            ctx.contract().restake(U32(alice_jar.id));
        });

        ctx.switch_account(carol());
        expect_panic(&ctx, "Account 'carol.near' doesn't exist", |ctx| {
            ctx.contract().restake(U32(alice_jar.id));
        });
    }

    #[test]
    #[should_panic(expected = "The product doesn't support restaking")]
    fn restake_when_restaking_is_not_supported() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("not_restakable_product").with_allows_restaking(false);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.switch_account(&alice);
        context.contract().restake(U32(jar.id));
    }

    #[test]
    #[should_panic(expected = "The jar is not mature yet")]
    fn restake_before_maturity() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product").with_allows_restaking(true);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.switch_account(&alice);
        context.contract().restake(U32(jar.id));
    }

    #[test]
    #[should_panic(expected = "The product is disabled")]
    fn restake_with_disabled_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product").with_allows_restaking(true);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin.clone())
            .with_products(&[product.clone()])
            .with_jars(&[jar.clone()]);

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| context.contract().set_enabled(product.id, false));

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract().restake(U32(jar.id));
    }

    #[test]
    #[should_panic(expected = "The jar is empty, nothing to restake")]
    fn restake_empty_jar() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product")
            .lockup_term(MS_IN_YEAR)
            .with_allows_restaking(true);
        let jar = Jar::generate(0, &alice, &product.id).principal(0);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract().restake(U32(jar.id));
    }

    #[test]
    fn restake_after_maturity_for_restakable_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product")
            .with_allows_restaking(true)
            .lockup_term(MS_IN_YEAR);
        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract().restake(U32(jar.id));

        let alice_jars = context.contract().get_jars_for_account(alice);

        assert_eq!(2, alice_jars.len());
        assert_eq!(0, alice_jars.iter().find(|item| item.id.0 == 0).unwrap().principal.0);
        assert_eq!(
            1_000_000,
            alice_jars.iter().find(|item| item.id.0 == 1).unwrap().principal.0
        );
    }

    #[test]
    #[should_panic(expected = "The product doesn't support restaking")]
    fn restake_after_maturity_for_not_restakable_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let reference_product = generate_product("not_restakable_product").with_allows_restaking(false);
        let jar = Jar::generate(0, &alice, &reference_product.id).principal(1_000_000);
        let mut context = Context::new(admin.clone())
            .with_products(&[reference_product.clone()])
            .with_jars(&[jar.clone()]);

        context.set_block_timestamp_in_days(366);

        context.switch_account(&alice);
        context.contract().restake(U32(jar.id));
    }

    #[test]
    #[should_panic(expected = "It's not possible to create new jars for this product")]
    fn create_jar_for_disabled_product() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product("restakable_product").enabled(false);
        let context = Context::new(admin).with_products(&[product.clone()]);

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context.contract().create_jar(alice, ticket, U128(1_000_000), None);
    }

    fn generate_premium_product(id: &str, signer: &MessageSigner) -> Product {
        Product::generate(id)
            .enabled(true)
            .public_key(signer.public_key())
            .cap(0, 100_000_000_000)
            .apy(Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }))
    }

    fn generate_product(id: &str) -> Product {
        Product::generate(id)
            .enabled(true)
            .cap(0, 100_000_000_000)
            .apy(Apy::Constant(UDecimal::new(20, 2)))
    }
}

mod helpers {
    use near_sdk::AccountId;
    use sweat_jar_model::TokenAmount;

    use crate::{common::Timestamp, jar::model::JarV1, Jar};

    impl Jar {
        pub(crate) fn generate(id: u32, account_id: &AccountId, product_id: &str) -> Jar {
            JarV1 {
                id,
                account_id: account_id.clone(),
                product_id: product_id.to_string(),
                created_at: 0,
                principal: 0,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                is_penalty_applied: false,
                claim_remainder: Default::default(),
            }
            .into()
        }

        pub(crate) fn principal(mut self, principal: TokenAmount) -> Jar {
            self.principal = principal;
            self
        }

        pub(crate) fn created_at(mut self, created_at: Timestamp) -> Jar {
            self.created_at = created_at;
            self
        }

        pub(crate) fn pending_withdraw(mut self) -> Jar {
            self.is_pending_withdraw = true;
            self
        }
    }
}
