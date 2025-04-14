use near_sdk::test_utils::test_env::{alice, bob, carol};
use sweat_jar_model::{
    api::{JarApi, ProductApi, RestakeApi},
    data::{deposit::{DepositMessage, DepositTicket}, jar::Jar, product::Product},
    signer::test_utils::MessageSigner,
    MS_IN_DAY, MS_IN_YEAR,
};

use crate::{
    common::tests::Context,
    test_utils::{admin, expect_panic, JarBuilder},
};

#[test]
fn restake_by_not_owner() {
    let product = Product::default();
    let alice_jar = Jar::new();
    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), alice_jar.clone())]);

    context.switch_account(bob());
    expect_panic(&context, "Account bob.near is not found", || {
        let valid_until = MS_IN_YEAR * 10;
        let ticket = DepositTicket {
            product_id: product.id.clone(),
            valid_until: valid_until.into(),
            timezone: None,
        };
        context.contract().restake(product.id.clone(), ticket, None, None);
    });

    expect_panic(&context, "Account bob.near is not found", || {
        let valid_until = MS_IN_YEAR * 10;
        let ticket = DepositTicket {
            product_id: product.id.clone(),
            valid_until: valid_until.into(),
            timezone: None,
        };
        context.contract().restake_all(ticket, None, None);
    });

    context.switch_account(carol());
    expect_panic(&context, "Account carol.near is not found", || {
        let valid_until = MS_IN_YEAR * 10;
        let ticket = DepositTicket {
            product_id: product.id.clone(),
            valid_until: valid_until.into(),
            timezone: None,
        };
        context.contract().restake(product.id.clone(), ticket, None, None);
    });

    expect_panic(&context, "Account carol.near is not found", || {
        let valid_until = MS_IN_YEAR * 10;
        let ticket = DepositTicket {
            product_id: product.id.clone(),
            valid_until: valid_until.into(),
            timezone: None,
        };
        context.contract().restake_all(ticket, None, None);
    });
}

#[test]
#[should_panic(expected = "Nothing to restake")]
fn restake_before_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let alice_jar = Jar::new();
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake(product.id, ticket, None, None);
}

#[test]
#[should_panic(expected = "It's not possible to create new jars for this product: the product is disabled.")]
fn restake_with_disabled_product() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let alice_jar = Jar::new();
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| context.contract().set_enabled(product.id.clone(), false));

    context.contract().products_cache.borrow_mut().clear();

    context.set_block_timestamp_in_days(366);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake(product.id, ticket, None, None);
}

#[test]
#[should_panic(expected = "Nothing to restake")]
fn restake_empty_jar() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let alice_jar = Jar::new();
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    context.set_block_timestamp_in_days(366);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake(product.id, ticket, None, None);
}

#[test]
fn restake_after_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let principal = 1_000_000;
    let alice_jar = Jar::new().with_deposit(0, principal);
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake(product.id, ticket, None, None);

    let alice_jars = context.contract().get_jars_for_account(alice);
    assert_eq!(1, alice_jars.len());

    let jar = alice_jars.first().unwrap();
    assert_eq!(principal, jar.principal.0);
    assert_eq!(restake_time, jar.created_at.0);
}

#[test]
fn restake_for_protected_product_success() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..Default::default()
    };
    let principal = 1_000_000;
    let alice_jar = Jar::new().with_deposit(0, principal);
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    let signature =
        signer.sign(DepositMessage::new(&context.owner, &alice, &product.id, principal, valid_until, 0).as_str());
    context
        .contract()
        .restake(product.id, ticket, Some(signature.into()), None);

    let alice_jars = context.contract().get_jars_for_account(alice);
    assert_eq!(1, alice_jars.len());

    let jar = alice_jars.first().unwrap();
    assert_eq!(principal, jar.principal.0);
    assert_eq!(restake_time, jar.created_at.0);
}

#[test]
#[should_panic(expected = "Not matching signature")]
fn restake_for_protected_product_invalid_signature() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..Default::default()
    };
    let principal = 1_000_000;
    let alice_jar = Jar::new().with_deposit(0, principal);
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };

    // invalid signature – wrong amount
    let signature =
        signer.sign(DepositMessage::new(&context.owner, &alice, &product.id, principal + 100, valid_until, 0).as_str());
    context
        .contract()
        .restake(product.id, ticket, Some(signature.into()), None);
}

#[test]
#[should_panic(expected = "Not matching signature")]
fn restake_for_protected_product_repeated_nonce() {
    let alice = alice();
    let admin = admin();

    let signer_1 = MessageSigner::new();
    let product_1 = Product {
        id: "product_1".to_string(),
        public_key: Some(signer_1.public_key().into()),
        ..Default::default()
    };

    let signer_2 = MessageSigner::new();
    let product_2 = Product {
        id: "product_2".to_string(),
        public_key: Some(signer_2.public_key().into()),
        ..Default::default()
    };

    let principal = 1_000_000;
    let alice_jar = Jar::new().with_deposit(0, principal);

    let mut context = Context::new(admin.clone())
        .with_products(&[product_1.clone(), product_2.clone()])
        .with_jars(
            &alice,
            &[
                (product_1.id.clone(), alice_jar.clone()),
                (product_2.id.clone(), alice_jar.clone()),
            ],
        );

    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product_1.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };

    let signature =
        signer_1.sign(DepositMessage::new(&context.owner, &alice, &product_1.id, principal, valid_until, 0).as_str());
    context
        .contract()
        .restake(product_1.id, ticket.clone(), Some(signature.into()), None);

    // invalid signature – repeated nonce
    let signature =
        signer_2.sign(DepositMessage::new(&context.owner, &alice, &product_2.id, principal, valid_until, 1).as_str());
    context
        .contract()
        .restake(product_2.id, ticket, Some(signature.into()), None);
}

#[test]
#[should_panic(expected = "Not matching signature")]
fn restake_for_protected_product_maturity_mistiming() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product {
        id: "product_1".to_string(),
        public_key: Some(signer.public_key().into()),
        ..Default::default()
    };

    let principal_1 = 1_000_000;
    let alice_jar_1 = Jar::new().with_deposit(0, principal_1);

    let principal_2 = 2_000;
    let alice_jar_2 = Jar::new().with_deposit(MS_IN_DAY * 2, principal_2);

    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]).with_jars(
        &alice,
        &[
            (product.id.clone(), alice_jar_1.clone()),
            (product.id.clone(), alice_jar_2.clone()),
        ],
    );

    // at this point the first deposit is mature
    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };

    // create signature for principal of first deposit only
    let signature =
        signer.sign(DepositMessage::new(&context.owner, &alice, &product.id, principal_1, valid_until, 0).as_str());

    // at this point both deposits are mature
    let restake_time = restake_time + 2 * MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context
        .contract()
        .restake(product.id, ticket.clone(), Some(signature.into()), None);
}
