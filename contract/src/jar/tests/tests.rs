#![cfg(test)]

use near_sdk::{
    json_types::U64,
    test_utils::test_env::alice,
};

use crate::{common::tests::Context, jar::model::JarTicket, product::model::Product, test_utils::admin};

#[test]
#[should_panic(expected = "It's not possible to create new jars for this product")]
fn create_jar_for_disabled_product() {
    let alice = alice();
    let admin = admin();

    let product = Product::new().enabled(false);
    let context = Context::new(admin).with_products(&[product.clone()]);

    let ticket = JarTicket {
        product_id: product.id,
        valid_until: U64(0),
        timezone: None,
    };

    context.contract().deposit(alice, ticket, 1_000_000, &None);
}

#[cfg(test)]
mod signature_tests {
    use near_sdk::{
        json_types::{Base64VecU8, U64},
        test_utils::test_env::alice,
    };

    use crate::{
        common::tests::Context,
        jar::model::JarTicket,
        product::{helpers::MessageSigner, model::Product},
        test_utils::{admin, generate_premium_product},
    };

    #[test]
    fn verify_ticket_with_valid_signature_and_date() {
        let admin = admin();

        let signer = MessageSigner::new();
        let product = generate_premium_product("premium_product", &signer);
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 14_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(123000000),
            timezone: None,
        };

        let signature = signer.sign(context.get_signature_material(&admin, &ticket, amount).as_str());

        context
            .contract()
            .verify(&admin, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Signature must be 64 bytes")]
    fn verify_ticket_with_invalid_signature() {
        let alice = alice();
        let admin = admin();

        let signer = MessageSigner::new();
        let product = generate_premium_product("premium_product", &signer);
        let context = Context::new(admin).with_products(&[product.clone()]);

        let amount = 1_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(100000000),
            timezone: None,
        };

        let signature: Vec<u8> = vec![0, 1, 2];

        context
            .contract()
            .verify(&alice, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Not matching signature")]
    fn verify_ticket_with_not_matching_signature() {
        let admin = admin();

        let signer = MessageSigner::new();
        let product = generate_premium_product("premium_product", &signer);
        let another_product = generate_premium_product("another_premium_product", &MessageSigner::new());

        let context = Context::new(admin.clone()).with_products(&[product, another_product.clone()]);

        let amount = 15_000_000;
        let ticket_for_another_product = JarTicket {
            product_id: another_product.id,
            valid_until: U64(100000000),
            timezone: None,
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
            &Some(Base64VecU8(signature)),
        );
    }

    #[test]
    #[should_panic(expected = "Ticket is outdated")]
    fn verify_ticket_with_invalid_date() {
        let alice = alice();
        let admin = admin();

        let signer = MessageSigner::new();
        let product = generate_premium_product("premium_product", &signer);
        let mut context = Context::new(admin).with_products(&[product.clone()]);

        context.set_block_timestamp_in_days(365);

        let amount = 5_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(100000000),
            timezone: None,
        };

        let signature = signer.sign(context.get_signature_material(&alice, &ticket, amount).as_str());

        context
            .contract()
            .verify(&alice, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Product not_existing_product is not found")]
    fn verify_ticket_with_not_existing_product() {
        let admin = admin();

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);

        let signer = MessageSigner::new();
        let not_existing_product = generate_premium_product("not_existing_product", &signer);

        let amount = 500_000;
        let ticket = JarTicket {
            product_id: not_existing_product.id,
            valid_until: U64(100000000),
            timezone: None,
        };

        let signature = signer.sign(context.get_signature_material(&admin, &ticket, amount).as_str());

        context
            .contract()
            .verify(&admin, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[test]
    #[should_panic(expected = "Signature is required")]
    fn verify_ticket_without_signature_when_required() {
        let admin = admin();

        let signer = MessageSigner::new();
        let product = generate_premium_product("not_existing_product", &signer);
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 3_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(100000000),
            timezone: None,
        };

        context.contract().verify(&admin, amount, &ticket, &None);
    }

    #[test]
    fn verify_ticket_without_signature_when_not_required() {
        let admin = admin();

        let product = Product::new();
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 4_000_000_000;
        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
            timezone: None,
        };

        context.contract().verify(&admin, amount, &ticket, &None);
    }
}
