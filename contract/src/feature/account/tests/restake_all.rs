use near_sdk::{test_utils::test_env::alice, AccountId};
use rstest::rstest;
use sweat_jar_model::{
    api::{ProductApi, RestakeApi},
    data::{
        deposit::{DepositMessage, DepositTicket},
        jar::Jar,
        product::{Product, ProductModelApi},
    },
    MS_IN_DAY, MS_IN_YEAR,
};

use crate::{
    common::{
        event::EventKind,
        testing::{accounts::*, Context},
    },
    feature::{account::model::test_utils::jar, product::model::test_utils::*},
};

#[rstest]
fn restake_all_for_single_product(
    admin: AccountId,
    #[from(product_1_year_apy_20_percent)] product: Product,
    #[with(vec![(0, 100_000), (MS_IN_YEAR / 4, 100_000), (MS_IN_YEAR / 2, 100_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar.clone())]);

    let test_time = MS_IN_YEAR * 6 / 4;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());

    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake_all(ticket, None, None);

    let contract = context.contract();
    let account = contract.get_account(&alice());
    let jar = account.get_jar(&product.id);
    assert_eq!(2, jar.deposits.len());
    assert_eq!(test_time, jar.deposits.last().unwrap().created_at);
    assert_eq!(200_000, jar.deposits.last().unwrap().principal);
    assert_eq!(test_time, jar.cache.unwrap().updated_at);
    assert_eq!(60_000, jar.cache.unwrap().interest);
}

#[rstest]
fn restake_all_for_different_products(
    admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
    #[from(product_1_year_apy_20_percent)] another_product: Product,
    #[with(vec![(0, 100_000), (MS_IN_YEAR / 2, 100_000)])]
    #[from(jar)]
    jar: Jar,
    #[with(vec![(0, 200_000), (MS_IN_YEAR / 2, 200_000)])]
    #[from(jar)]
    another_jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone(), another_product.clone()])
        .with_jars(
            &alice(),
            &[
                (product.id.clone(), jar.clone()),
                (another_product.id.clone(), another_jar.clone()),
            ],
        );

    let test_time = MS_IN_YEAR * 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: another_product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake_all(ticket, None, None);

    let contract = context.contract();
    let account = contract.get_account(&alice());

    let jar = account.get_jar(&product.id);
    assert_eq!(0, jar.deposits.len());
    assert_eq!(test_time, jar.cache.unwrap().updated_at);
    assert_eq!(20_000, jar.cache.unwrap().interest);

    let another_jar = account.get_jar(&another_product.id);
    assert_eq!(1, another_jar.deposits.len());
    assert_eq!(test_time, another_jar.deposits.last().unwrap().created_at);
    assert_eq!(600_000, another_jar.deposits.last().unwrap().principal);
    assert_eq!(test_time, another_jar.cache.unwrap().updated_at);
    assert_eq!(80_000, another_jar.cache.unwrap().interest);
}

#[rstest]
fn restake_all_to_new_product(
    admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
    #[from(product_1_year_apy_20_percent)] another_product: Product,
    #[with(vec![(0, 50_000), (MS_IN_YEAR / 4, 20_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone(), another_product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 3 / 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: another_product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake_all(ticket, None, None);

    let contract = context.contract();
    let account = contract.get_account(&alice());

    let jar = account.get_jar(&product.id);
    assert_eq!(0, jar.deposits.len());
    assert_eq!(test_time, jar.cache.unwrap().updated_at);
    assert_eq!(7_000, jar.cache.unwrap().interest);

    let another_jar = account.get_jar(&another_product.id);
    assert_eq!(1, another_jar.deposits.len());
    assert_eq!(test_time, another_jar.deposits.last().unwrap().created_at);
    assert_eq!(70_000, another_jar.deposits.last().unwrap().principal);
    assert!(another_jar.cache.is_none());
}

#[rstest]
#[should_panic(expected = "Product not_existing_product is not found")]
fn restake_all_to_not_existing_product(
    admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
    #[with(vec![(0, 500_000), (MS_IN_YEAR / 5, 700_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 3 / 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: "not_existing_product".into(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake_all(ticket, None, None);
}

#[rstest]
#[should_panic(expected = "It's not possible to create new jars for this product")]
fn restake_all_to_disabled_product(
    admin: AccountId,
    #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer }: ProtectedProduct,
    #[with(vec![(0, 150_000), (MS_IN_YEAR / 3, 770_000)])] jar: Jar,
) {
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar.clone())]);

    let test_time = MS_IN_YEAR * 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(admin);
    context.with_deposit_yocto(1, |context| context.contract().set_enabled(product.id.clone(), false));

    context.switch_account(alice());
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    let message = DepositMessage::new(
        &context.owner,
        &alice(),
        &product.id,
        jar.total_principal(),
        valid_until,
        0,
    );
    let signature = signer.sign(message.as_str());

    context.contract().restake_all(ticket, Some(signature.into()), None);
}

#[rstest]
fn restake_all_with_withdrawal(
    admin: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
    #[with(vec![(0, 200_000), (MS_IN_YEAR / 4, 800_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };
    context.contract().restake_all(ticket, None, Some(100_000.into()));

    let contract = context.contract();
    let account = contract.get_account(&alice());
    let jar = account.get_jar(&product.id);
    assert_eq!(1, jar.deposits.len());
    assert_eq!(test_time, jar.deposits.last().unwrap().created_at);
    assert_eq!(100_000, jar.deposits.last().unwrap().principal);
    assert_eq!(test_time, jar.cache.unwrap().updated_at);
    assert_eq!(100_000, jar.cache.unwrap().interest);
}

#[rstest]
fn restake_all_for_multiple_products_with_withdrawal(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_apy_10_percent)] product: Product,
    #[from(product_1_year_apy_20_percent)] another_product: Product,
    #[with(vec![(0, 200_000), (MS_IN_YEAR / 4, 300_000)])]
    #[from(jar)]
    jar: Jar,
    #[with(vec![(0, 400_000), (MS_IN_YEAR / 4, 500_000)])]
    #[from(jar)]
    another_jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone(), another_product.clone()])
        .with_jars(
            &alice,
            &[
                (product.id.clone(), jar.clone()),
                (another_product.id.clone(), another_jar.clone()),
            ],
        );

    // Wait until maturity
    let restake_time = 2 * MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    // Create restake ticket
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };

    let principal = &jar.total_principal() + &another_jar.total_principal();
    let withdrawal_amount = 200_000;
    context.switch_account(&alice);
    context
        .contract()
        .restake_all(ticket, None, Some((principal - withdrawal_amount).into()));

    // Check emitted event
    let events = context.get_events();
    assert_eq!(events.len(), 1);

    let EventKind::Restake(_, data) = events.last().unwrap() else {
        panic!("Expected Restake event");
    };
    assert_eq!(data.restaked.0, principal - withdrawal_amount);
    assert_eq!(data.withdrawn.0, withdrawal_amount);
}

#[rstest]
fn restake_all_for_multiple_products_with_withdrawal_and_fee(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent_with_fixed_fee)] product: Product,
    #[from(product_1_year_12_percent_with_percent_fee)] another_product: Product,
    #[with(vec![(0, 200_000), (MS_IN_YEAR / 4, 300_000)])]
    #[from(jar)]
    jar: Jar,
    #[with(vec![(0, 400_000), (MS_IN_YEAR / 4, 500_000)])]
    #[from(jar)]
    another_jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone(), another_product.clone()])
        .with_jars(
            &alice,
            &[
                (product.id.clone(), jar.clone()),
                (another_product.id.clone(), another_jar.clone()),
            ],
        );

    // Wait until maturity
    let restake_time = 2 * MS_IN_YEAR + MS_IN_DAY;
    context.set_block_timestamp_in_ms(restake_time);

    // Create restake ticket
    let valid_until = MS_IN_YEAR * 10;
    let ticket = DepositTicket {
        product_id: product.id.clone(),
        valid_until: valid_until.into(),
        timezone: None,
    };

    let principal = &jar.total_principal() + &another_jar.total_principal();
    let total_fee =
        product.calculate_fee(jar.total_principal()) + another_product.calculate_fee(another_jar.total_principal());
    let withdrawal_amount = 100_000;
    let target_fee = (total_fee * withdrawal_amount).div_ceil(principal);
    context.switch_account(&alice);
    context
        .contract()
        .restake_all(ticket, None, Some((principal - withdrawal_amount).into()));

    // Check emitted event
    let events = context.get_events();
    assert_eq!(events.len(), 1);

    let EventKind::Restake(_, data) = events.last().unwrap() else {
        panic!("Expected Restake event");
    };
    assert_eq!(data.restaked.0, principal - withdrawal_amount);
    assert_eq!(data.withdrawn.0, withdrawal_amount - target_fee);
    assert_eq!(context.contract().fee_amount, target_fee);
}
