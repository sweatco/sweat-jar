use near_sdk::AccountId;
use rstest::rstest;
use sweat_jar_model::{
    api::{AccountApi, ProductApi, RestakeApi},
    data::{
        deposit::{DepositMessage, DepositTicket},
        jar::Jar,
        product::Product,
    },
    TokenAmount, MS_IN_DAY, MS_IN_YEAR,
};

use crate::{
    common::testing::{
        accounts::{admin, alice, bob, carol},
        expect_panic, Context,
    },
    feature::{
        account::model::test_utils::jar,
        product::model::test_utils::{product, protected_product, ProtectedProduct},
    },
};

#[rstest]
fn restake_by_not_owner(admin: AccountId, bob: AccountId, product: Product, #[from(jar)] alice_jar: Jar) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), alice_jar.clone())]);

    context.switch_account(bob);
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

#[rstest]
#[should_panic(expected = "Nothing to restake")]
fn restake_before_maturity(alice: AccountId, admin: AccountId, product: Product, #[from(jar)] alice_jar: Jar) {
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

#[rstest]
#[should_panic(expected = "It's not possible to create new jars for this product: the product is disabled.")]
fn restake_with_disabled_product(alice: AccountId, admin: AccountId, product: Product, #[from(jar)] alice_jar: Jar) {
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

#[rstest]
#[should_panic(expected = "Nothing to restake")]
fn restake_empty_jar(alice: AccountId, admin: AccountId, product: Product, #[from(jar)] alice_jar: Jar) {
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

#[rstest]
fn restake_after_maturity(
    alice: AccountId,
    admin: AccountId,
    product: Product,
    #[values(100, 100_000, 2_500_000)] principal: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal)])]
    alice_jar: Jar,
) {
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
    context.contract().restake(product.id.clone(), ticket, None, None);

    let alice_jars = context.contract().get_jars_for_account(alice);
    assert_eq!(1, alice_jars.0.get(&product.id).unwrap().len());

    let jar = alice_jars.0.get(&product.id).unwrap().first().unwrap();
    assert_eq!(principal, jar.1.into());
    assert_eq!(restake_time, jar.0);
}

#[rstest]
fn restake_for_protected_product_success(
    alice: AccountId,
    admin: AccountId,
    #[from(protected_product)] ProtectedProduct { product, signer }: ProtectedProduct,
    #[values(100, 100_000, 2_500_000)] principal: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal)])]
    alice_jar: Jar,
) {
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
    let signature = signer
        .sign(DepositMessage::new(&context.owner, &alice, &product.id.clone(), principal, valid_until, 0).as_str());
    context
        .contract()
        .restake(product.id.clone(), ticket, Some(signature.into()), None);

    let alice_jars = context.contract().get_jars_for_account(alice);
    assert_eq!(1, alice_jars.0.len());
    assert_eq!(1, alice_jars.0.get(&product.id.clone()).unwrap().len());

    let jar = alice_jars.0.get(&product.id.clone()).unwrap().first().unwrap();
    assert_eq!(principal, jar.1.into());
    assert_eq!(restake_time, jar.0);
}

#[rstest]
fn sequential_restake_for_protected_product_success(
    alice: AccountId,
    admin: AccountId,
    #[from(protected_product)] ProtectedProduct { product, signer }: ProtectedProduct,
    #[values(100, 100_000, 2_500_000)] principal: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal)])]
    alice_jar: Jar,
) {
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

    let signature = signer
        .sign(DepositMessage::new(&context.owner, &alice, &product.id.clone(), principal, valid_until, 0).as_str());
    context
        .contract()
        .restake(product.id.clone(), ticket.clone(), Some(signature.into()), None);

    let alice_jars = context.contract().get_jars_for_account(alice.clone());
    assert_eq!(1, alice_jars.0.len());
    assert_eq!(1, alice_jars.0.get(&product.id.clone()).unwrap().len());

    let jar = alice_jars.0.get(&product.id.clone()).unwrap().first().unwrap();
    assert_eq!(principal, jar.1.into());
    assert_eq!(restake_time, jar.0);

    let restake_time = restake_time + MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    let signature = signer
        .sign(DepositMessage::new(&context.owner, &alice, &product.id.clone(), principal, valid_until, 1).as_str());
    context
        .contract()
        .restake(product.id.clone(), ticket.clone(), Some(signature.into()), None);

    let alice_jars = context.contract().get_jars_for_account(alice.clone());
    let alice_jars = context.contract().get_jars_for_account(alice.clone());
    assert_eq!(1, alice_jars.0.len());
    assert_eq!(1, alice_jars.0.get(&product.id.clone()).unwrap().len());

    let jar = alice_jars.0.get(&product.id.clone()).unwrap().first().unwrap();
    assert_eq!(principal, jar.1.into());
    assert_eq!(restake_time, jar.0);
}

#[rstest]
#[should_panic(expected = "Not matching signature")]
fn restake_for_protected_product_invalid_signature(
    alice: AccountId,
    admin: AccountId,
    #[from(protected_product)] ProtectedProduct { product, signer }: ProtectedProduct,
    #[values(100, 100_000, 2_500_000)] principal: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal)])]
    alice_jar: Jar,
) {
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

#[rstest]
#[should_panic(expected = "Not matching signature")]
fn restake_for_protected_product_repeated_nonce(
    alice: AccountId,
    admin: AccountId,
    #[from(protected_product)]
    #[with("product_1".to_string())]
    ProtectedProduct {
        product: product_1,
        signer: signer_1,
    }: ProtectedProduct,
    #[from(protected_product)]
    #[with("product_2".to_string())]
    ProtectedProduct {
        product: product_2,
        signer: signer_2,
    }: ProtectedProduct,
    #[values(100, 100_000, 2_500_000)] principal: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal)])]
    alice_jar: Jar,
) {
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

#[rstest]
#[should_panic(expected = "Not matching signature")]
fn restake_for_protected_product_maturity_mistiming(
    alice: AccountId,
    admin: AccountId,
    #[from(protected_product)] ProtectedProduct { product, signer }: ProtectedProduct,
    #[values(100, 100_000, 2_500_000)] principal_1: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal_1)])]
    alice_jar_1: Jar,
    #[values(150_000, 7_000_000, 9_500_000)] _principal_2: TokenAmount,
    #[from(jar)]
    #[with(vec![(MS_IN_DAY * 2, _principal_2)])]
    alice_jar_2: Jar,
) {
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

#[rstest]
#[should_panic(expected = "Not matching signature")]
fn deposit_with_outdated_nonce_after_restake(
    alice: AccountId,
    admin: AccountId,
    #[from(protected_product)] ProtectedProduct { product, signer }: ProtectedProduct,
    #[values(100_000)] principal: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal)])]
    alice_jar: Jar,
) {
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar.clone())]);

    // Wait until maturity
    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    context.switch_account(&alice);
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };

    // Create signature for restake
    let nonce = 0;
    let signature =
        signer.sign(DepositMessage::new(&context.owner, &alice, &product.id, principal, valid_until, nonce).as_str());

    // Perform restake which should increment nonce
    context
        .contract()
        .restake(product.id.clone(), ticket.clone(), Some(signature.into()), None);

    // Try to create new jar with outdated nonce (0)
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    let signature =
        signer.sign(DepositMessage::new(&context.owner, &alice, &product.id, principal, valid_until, nonce).as_str());

    context
        .contract()
        .deposit(alice, ticket, principal, &Some(signature.into()));
}

#[rstest]
fn restake_with_withdrawal(
    admin: AccountId,
    alice: AccountId,
    #[from(product)] product: Product,
    #[values(1_000_000)] principal: TokenAmount,
    #[from(jar)]
    #[with(vec![(0, principal)])]
    alice_jar: Jar,
) {
    use crate::common::event::EventKind;

    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar)]);

    // Wait until maturity
    let restake_time = MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    // Create restake ticket
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };

    let withdrawal_amount = 100;
    context.switch_account(&alice);
    context.contract().restake(
        product.id.clone(),
        ticket,
        None,
        Some((principal - withdrawal_amount).into()),
    );

    // Check emitted event
    let events = context.get_events();
    assert_eq!(events.len(), 1);

    let EventKind::Restake(_, data) = events.last().unwrap() else {
        panic!("Expected Restake event");
    };
    assert_eq!(data.restaked.0, principal - withdrawal_amount);
    assert_eq!(data.withdrawn.0, withdrawal_amount);
}
