#![cfg(test)]

use fake::Fake;
use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    AccountId,
};
use rstest::rstest;
use sweat_jar_model::{
    api::ProductApi,
    data::{
        account::Account,
        jar::Jar,
        product::{
            Apy, Cap, DowngradableApy, FixedProductTerms, FlexibleProductTerms, Product, ProductAssertions,
            ProductModelApi, ScoreBasedProductTerms, Terms, WithdrawalFee,
        },
    },
    interest::InterestCalculator,
    signer::test_utils::MessageSigner,
    Timestamp, UDecimal, MS_IN_YEAR,
};

use crate::{
    common::testing::{accounts::*, Context, TokenUtils},
    feature::{
        account::model::test_utils::jar,
        product::model::test_utils::{
            product, product_1_year_12_cap_score_based, product_1_year_12_percent,
            product_1_year_12_percent_with_fixed_fee, product_1_year_12_percent_with_invalid_fixed_fee,
            product_1_year_12_percent_with_invalid_percent_fee, product_1_year_12_percent_with_percent_fee,
            product_1_year_30_cap_score_based_protected, product_1_year_apy_7_percent_protected,
            product_1_year_apy_downgradable_20_10_percent_protected, product_2_years_10_percent,
            product_flexible_10_percent, BaseApy, ProductBuilder, ProtectedProduct,
        },
    },
};

#[rstest]
fn add_product_to_list_by_admin(admin: AccountId, product: Product) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));

    let products = context.contract().get_products();
    assert_eq!(products.len(), 1);
    assert_eq!(products.first().unwrap().id, product.id.to_string());
}

#[rstest]
#[should_panic(expected = "Can be performed only by admin")]
fn add_product_to_list_by_not_admin(admin: AccountId, product: Product) {
    let mut context = Context::new(admin);

    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));
}

#[rstest]
fn disable_product_when_enabled(admin: AccountId, product: Product) {
    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let mut product = context.contract().get_product(&product.id);
    assert!(product.is_enabled);

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| {
        context.contract().set_enabled(product.id.to_string(), false)
    });

    context.contract().products_cache.borrow_mut().clear();

    product = context.contract().get_product(&product.id);
    assert!(!product.is_enabled);
}

#[rstest]
#[should_panic(expected = "Status matches")]
fn enable_product_when_enabled(admin: AccountId, product: Product) {
    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let product = context.contract().get_product(&product.id);
    assert!(product.is_enabled);

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| {
        context.contract().set_enabled(product.id.to_string(), true)
    });
}

#[rstest]
#[should_panic(expected = "Product already exists")]
fn register_product_with_existing_id(admin: AccountId, product: Product) {
    let mut context = Context::new(admin.clone());

    context.switch_account(&admin);

    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));

    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));
}

#[rstest]
fn register_downgradable_product(
    admin: AccountId,
    #[from(product_1_year_apy_downgradable_20_10_percent_protected)]
    ProtectedProduct { product, signer: _ }: ProtectedProduct,
) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product));

    let product = context.contract().get_products().first().unwrap().clone();

    assert_eq!(
        product.get_base_apy().clone(),
        Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(20_000, 5),
            fallback: UDecimal::new(10_000, 5),
        })
    );
}

#[rstest]
#[should_panic(
    expected = "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
)]
fn register_product_with_too_high_fixed_fee(
    admin: AccountId,
    #[from(product_1_year_12_percent_with_invalid_fixed_fee)] product: Product,
) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product));
}

#[rstest]
#[should_panic(
    expected = "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
)]
fn register_product_with_too_high_percent_fee(
    admin: AccountId,
    #[from(product_1_year_12_percent_with_invalid_percent_fee)] product: Product,
) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product));
}

#[rstest]
fn register_product_with_fee(
    admin: AccountId,
    #[from(product_1_year_12_percent_with_fixed_fee)] product_with_fixed_fee: Product,
    #[from(product_1_year_12_percent_with_percent_fee)] product_with_percent_fee: Product,
) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product_with_fixed_fee));

    let product = context.contract().get_products().first().unwrap().clone();
    assert_eq!(product.withdrawal_fee, Some(WithdrawalFee::Fix(100.into())));

    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| {
        context.contract().register_product(product_with_percent_fee)
    });

    let product = context.contract().get_products().first().unwrap().clone();
    assert_eq!(
        product.withdrawal_fee,
        Some(WithdrawalFee::Percent(UDecimal::new(10_000, 5)))
    );
}

#[rstest]
fn register_product_with_flexible_terms(admin: AccountId, #[from(product_flexible_10_percent)] product: Product) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product));

    let product = context.contract().get_products().first().unwrap().clone();

    assert!(matches!(product.terms, Terms::Flexible(_)));
}

#[rstest]
fn set_public_key(
    admin: AccountId,
    #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer: _ }: ProtectedProduct,
) {
    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let new_signer = MessageSigner::new();
    let new_pk = new_signer.public_key();

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| {
        context
            .contract()
            .set_public_key(product.id.clone(), Base64VecU8(new_pk.clone()))
    });

    let product = context.contract().products.get(&product.id).unwrap();
    assert_eq!(&new_pk, product.get_public_key().as_ref().unwrap());
}

#[rstest]
#[should_panic(expected = "Can be performed only by admin")]
fn set_public_key_by_not_admin(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer: _ }: ProtectedProduct,
) {
    let mut context = Context::new(admin).with_products(&[product.clone()]);

    let new_signer = MessageSigner::new();
    let new_pk = new_signer.public_key();

    context.switch_account(&alice);
    context.with_deposit_yocto(1, |context| {
        context.contract().set_public_key(product.id, Base64VecU8(new_pk))
    });
}

#[rstest]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
fn set_public_key_without_deposit(
    admin: AccountId,
    #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer: _ }: ProtectedProduct,
) {
    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let new_signer = MessageSigner::new();
    let new_pk = new_signer.public_key();

    context.switch_account(&admin);

    context.contract().set_public_key(product.id, Base64VecU8(new_pk));
}

#[rstest]
fn assert_cap_in_bounds(#[from(product_1_year_12_percent_with_fixed_fee)] product: Product) {
    product.assert_cap(2_000);
}

#[rstest]
#[should_panic(expected = "Total amount is out of product bounds: [1000..1000000000000000000000000000]")]
fn assert_cap_less_than_min(#[from(product_1_year_12_percent_with_fixed_fee)] product: Product) {
    product.assert_cap(10);
}

#[rstest]
#[should_panic(expected = "Total amount is out of product bounds: [1000..1000000000000000000000000000]")]
fn assert_cap_more_than_max(#[from(product_1_year_12_percent_with_fixed_fee)] product: Product) {
    product.assert_cap(500_000_000_000.to_otto());
}

#[rstest]
fn get_interest_before_maturity(
    #[from(product_2_years_10_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
    let account = Account::default();

    let (interest, _) = product.terms.get_interest(&account, &jar, MS_IN_YEAR);
    assert_eq!(10_000_000, interest);
}

#[rstest]
fn get_interest_after_maturity(
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
    let account = Account::default();

    let (interest, _) = product.terms.get_interest(&account, &jar, 400 * 24 * 60 * 60 * 1000);
    assert_eq!(12_000_000, interest);
}

#[rstest]
fn interest_precision(#[with(vec![(0, u128::from(MS_IN_YEAR))])] jar: Jar) {
    let terms = Terms::Fixed(FixedProductTerms {
        apy: Apy::Constant(UDecimal::new(1, 0)),
        lockup_term: MS_IN_YEAR.into(),
    });
    let account = Account::default();

    assert_eq!(terms.get_interest(&account, &jar, 10000000000).0, 10000000000);
    assert_eq!(terms.get_interest(&account, &jar, 10000000001).0, 10000000001);

    for _ in 0..100 {
        let time: Timestamp = (10..MS_IN_YEAR).fake();
        assert_eq!(terms.get_interest(&account, &jar, time).0, time as u128);
    }
}

#[rstest]
fn register_score_based_product_with_signature(
    admin: AccountId,
    #[from(product_1_year_30_cap_score_based_protected)] ProtectedProduct { product, signer: _ }: ProtectedProduct,
) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));

    assert_eq!(product.id, context.contract().get_products().first().unwrap().id);
}

#[rstest]
#[should_panic(expected = "Score based must be protected.")]
fn register_score_based_product_without_signature(
    admin: AccountId,
    #[from(product_1_year_12_cap_score_based)] product: Product,
) {
    let mut context = Context::new(admin.clone());

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));
}

#[rstest]
#[should_panic(expected = "Cap minimum must be less than maximum")]
fn register_product_with_inverted_cap(admin: AccountId, #[from(product_1_year_12_percent)] product: Product) {
    let mut context = Context::new(admin.clone());
    let product = product.with_cap(1_000_000, 100);

    context.switch_account_to_manager();
    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));
}
