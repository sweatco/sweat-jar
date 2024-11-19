use near_sdk::test_utils::test_env::alice;
use sweat_jar_model::{
    api::{ProductApi, RestakeApi},
    UDecimal, MS_IN_YEAR,
};

use crate::{
    common::tests::Context,
    jar::model::JarV2,
    product::model::{Apy, FixedProductTerms, ProductV2, Terms},
    test_utils::admin,
};

#[test]
fn restake_all_for_single_product() {
    let product = ProductV2::new();
    let jar = JarV2::new().with_deposits(vec![(0, 100_000), (MS_IN_YEAR / 4, 100_000), (MS_IN_YEAR / 2, 100_000)]);
    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 6 / 4;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(&alice());
    context.contract().restake_all(product.id.clone(), None);

    let contract = context.contract();
    let account = contract.get_account(&alice());
    let jar = account.get_jar(&product.id);
    assert_eq!(2, jar.deposits.len());
    assert_eq!(test_time, jar.deposits.last().unwrap().created_at);
    assert_eq!(200_000, jar.deposits.last().unwrap().principal);
    assert_eq!(test_time, jar.cache.unwrap().updated_at);
    assert_eq!(60_000, jar.cache.unwrap().interest);
}

#[test]
fn restake_all_for_different_products() {
    let product = ProductV2::new().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR,
        apy: Apy::Constant(UDecimal::new(10_000, 5)),
    }));
    let another_product = ProductV2::new()
        .with_id("another_product".into())
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR,
            apy: Apy::Constant(UDecimal::new(20_000, 5)),
        }));
    let jar = JarV2::new().with_deposits(vec![(0, 100_000), (MS_IN_YEAR / 2, 100_000)]);
    let another_jar = JarV2::new().with_deposits(vec![(0, 200_000), (MS_IN_YEAR / 2, 200_000)]);
    let mut context = Context::new(admin())
        .with_products(&[product.clone(), another_product.clone()])
        .with_jars(
            &alice(),
            &[(product.id.clone(), jar), (another_product.id.clone(), another_jar)],
        );

    let test_time = MS_IN_YEAR * 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    context.contract().restake_all(another_product.id.clone(), None);

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

#[test]
fn restake_all_to_new_product() {
    let product = ProductV2::new().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR,
        apy: Apy::Constant(UDecimal::new(10_000, 5)),
    }));
    let another_product = ProductV2::new()
        .with_id("another_product".into())
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR,
            apy: Apy::Constant(UDecimal::new(20_000, 5)),
        }));
    let jar = JarV2::new().with_deposits(vec![(0, 50_000), (MS_IN_YEAR / 4, 20_000)]);
    let mut context = Context::new(admin())
        .with_products(&[product.clone(), another_product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 3 / 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    context.contract().restake_all(another_product.id.clone(), None);

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

#[test]
#[should_panic(expected = "Product not_existing_product is not found")]
fn restake_all_to_not_existing_product() {
    let product = ProductV2::new().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR,
        apy: Apy::Constant(UDecimal::new(10_000, 5)),
    }));
    let jar = JarV2::new().with_deposits(vec![(0, 500_000), (MS_IN_YEAR / 5, 700_000)]);
    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 3 / 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    context.contract().restake_all("not_existing_product".into(), None);
}

#[test]
#[should_panic(expected = "It's not possible to create new jars for this product")]
fn restake_all_to_disabled_product() {
    let product = ProductV2::new().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR,
        apy: Apy::Constant(UDecimal::new(7_000, 5)),
    }));
    let jar = JarV2::new().with_deposits(vec![(0, 150_000), (MS_IN_YEAR / 3, 770_000)]);
    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(admin());
    context.with_deposit_yocto(1, |context| context.contract().set_enabled(product.id.clone(), false));

    context.switch_account(alice());
    context.contract().restake_all(product.id, None);
}

#[test]
fn restake_all_with_withdrawal() {
    let product = ProductV2::new().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR,
        apy: Apy::Constant(UDecimal::new(10_000, 5)),
    }));
    let jar = JarV2::new().with_deposits(vec![(0, 200_000), (MS_IN_YEAR / 4, 800_000)]);
    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar)]);

    let test_time = MS_IN_YEAR * 2;
    context.set_block_timestamp_in_ms(test_time);

    context.switch_account(alice());
    context.contract().restake_all(product.id.clone(), Some(100_000.into()));

    let contract = context.contract();
    let account = contract.get_account(&alice());
    let jar = account.get_jar(&product.id);
    assert_eq!(1, jar.deposits.len());
    assert_eq!(test_time, jar.deposits.last().unwrap().created_at);
    assert_eq!(100_000, jar.deposits.last().unwrap().principal);
    assert_eq!(test_time, jar.cache.unwrap().updated_at);
    assert_eq!(100_000, jar.cache.unwrap().interest);
}
