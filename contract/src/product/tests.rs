#![cfg(test)]

use fake::Fake;
use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    test_utils::test_env::alice,
};
use sweat_jar_model::{
    api::ProductApi,
    data::product::{
        Apy, Cap, DowngradableApy, FixedProductTerms, FlexibleProductTerms, Product, ScoreBasedProductTerms, Terms,
        WithdrawalFee,
    },
    signer::test_utils::MessageSigner,
    UDecimal, MS_IN_YEAR,
};

use crate::{
    common::{
        tests::{Context, TokenUtils},
        Timestamp,
    },
    jar::{account::Account, model::Jar},
    product::model::v1::{InterestCalculator, ProductAssertions, ProductModelApi},
    test_utils::admin,
};

pub(crate) fn get_testing_product() -> Product {
    Product {
        id: "product".to_string(),
        ..Default::default()
    }
}

#[test]
fn disable_product_when_enabled() {
    let admin = admin();
    let product = &Product::default();

    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let mut product = context.contract().get_product(&product.id);
    assert!(product.is_enabled);

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| {
        context.contract().set_enabled(product.id.to_string(), false)
    });

    context.contract().products_cache.borrow_mut().clear();

    product = context.contract().get_product(&product.id);
    assert!(!product.is_enabled);
}

#[test]
#[should_panic(expected = "Status matches")]
fn enable_product_when_enabled() {
    let admin = admin();
    let product = &Product::default();

    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let product = context.contract().get_product(&product.id);
    assert!(product.is_enabled);

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| {
        context.contract().set_enabled(product.id.to_string(), true)
    });
}

#[test]
#[should_panic(expected = "Product already exists")]
fn register_product_with_existing_id() {
    let admin = admin();

    let mut context = Context::new(admin.clone());

    context.switch_account(&admin);

    context.with_deposit_yocto(1, |context| {
        let first_product = get_testing_product();
        context.contract().register_product(first_product)
    });

    context.with_deposit_yocto(1, |context| {
        let second_product = get_testing_product();
        context.contract().register_product(second_product)
    });
}

#[test]
fn register_downgradable_product() {
    let product = register_product(Product {
        id: "downgradable_product".to_string(),
        terms: Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR.into(),
            apy: Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(12, 2),
                fallback: UDecimal::new(10, 3),
            }),
        }),
        ..Default::default()
    });

    assert_eq!(
        product.get_base_apy().clone(),
        Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(12, 2),
            fallback: UDecimal::new(10, 3),
        })
    );
}

#[test]
#[should_panic(
    expected = "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
)]
fn register_product_with_too_high_fixed_fee() {
    register_product(Product {
        id: "product_with_fixed_fee".to_string(),
        withdrawal_fee: WithdrawalFee::Fix(U128(200)).into(),
        terms: Terms::Fixed(FixedProductTerms {
            apy: Default::default(),
            lockup_term: U64(MS_IN_YEAR),
        }),
        ..Default::default()
    });
}

#[test]
#[should_panic(
    expected = "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
)]
fn register_product_with_too_high_percent_fee() {
    register_product(Product {
        id: "product_with_fixed_fee".to_string(),
        withdrawal_fee: WithdrawalFee::Percent(UDecimal::new(100, 0)).into(),
        ..Default::default()
    });
}

#[test]
fn register_product_with_fee() {
    let product = register_product(Product {
        id: "product_with_fixed_fee".to_string(),
        withdrawal_fee: WithdrawalFee::Fix(U128(10)).into(),
        cap: Cap::new(20, 10_000_000.to_otto()),
        ..Default::default()
    });

    assert_eq!(product.withdrawal_fee, Some(WithdrawalFee::Fix(10.into())));

    let product = register_product(Product {
        id: "product_with_percent_fee".to_string(),
        withdrawal_fee: WithdrawalFee::Percent(UDecimal::new(12, 2)).into(),
        ..Default::default()
    });

    assert_eq!(
        product.withdrawal_fee,
        Some(WithdrawalFee::Percent(UDecimal::new(12, 2)))
    );
}

#[test]
fn register_product_with_flexible_terms() {
    let product = register_product(Product {
        id: "product_with_fixed_fee".to_string(),
        terms: Terms::Flexible(FlexibleProductTerms { apy: Apy::default() }),
        ..Product::default()
    });

    assert!(matches!(product.terms, Terms::Flexible(_)));
}

#[test]
fn set_public_key() {
    let admin = admin();

    let signer = MessageSigner::new();
    let product = generate_product().with_public_key(signer.public_key().into());
    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let new_signer = MessageSigner::new();
    let new_pk = new_signer.public_key();

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| {
        context
            .contract()
            .set_public_key(product.id.clone(), Base64VecU8(new_pk.clone()))
    });

    let product = context.contract().products.get(&product.id).unwrap();
    assert_eq!(&new_pk, product.get_public_key().as_ref().unwrap());
}

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn set_public_key_by_not_admin() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = generate_product().with_public_key(signer.public_key().into());
    let mut context = Context::new(admin).with_products(&[product.clone()]);

    let new_signer = MessageSigner::new();
    let new_pk = new_signer.public_key();

    context.switch_account(&alice);
    context.with_deposit_yocto(1, |context| {
        context.contract().set_public_key(product.id, Base64VecU8(new_pk))
    });
}

#[test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
fn set_public_key_without_deposit() {
    let admin = admin();

    let signer = MessageSigner::new();
    let product = generate_product().with_public_key(signer.public_key().into());
    let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

    let new_signer = MessageSigner::new();
    let new_pk = new_signer.public_key();

    context.switch_account(&admin);

    context.contract().set_public_key(product.id, Base64VecU8(new_pk));
}

#[test]
fn assert_cap_in_bounds() {
    generate_product().assert_cap(200);
}

#[test]
#[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
fn assert_cap_less_than_min() {
    generate_product().assert_cap(10);
}

#[test]
#[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
fn assert_cap_more_than_max() {
    generate_product().assert_cap(500_000_000_000);
}

#[test]
fn get_interest_before_maturity() {
    let terms = Terms::Fixed(FixedProductTerms {
        apy: Apy::Constant(UDecimal::new(12, 2)),
        lockup_term: (2 * MS_IN_YEAR).into(),
    });
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let account = Account::default();

    let (interest, _) = terms.get_interest(&account, &jar, MS_IN_YEAR);
    assert_eq!(12_000_000, interest);
}

#[test]
fn get_interest_after_maturity() {
    let terms = Terms::Fixed(FixedProductTerms {
        apy: Apy::Constant(UDecimal::new(12, 2)),
        lockup_term: MS_IN_YEAR.into(),
    });
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let account = Account::default();

    let (interest, _) = terms.get_interest(&account, &jar, 400 * 24 * 60 * 60 * 1000);
    assert_eq!(12_000_000, interest);
}

#[test]
fn interest_precision() {
    let terms = Terms::Fixed(FixedProductTerms {
        apy: Apy::Constant(UDecimal::new(1, 0)),
        lockup_term: MS_IN_YEAR.into(),
    });
    let jar = Jar::new().with_deposit(0, u128::from(MS_IN_YEAR));
    let account = Account::default();

    assert_eq!(terms.get_interest(&account, &jar, 10000000000).0, 10000000000);
    assert_eq!(terms.get_interest(&account, &jar, 10000000001).0, 10000000001);

    for _ in 0..100 {
        let time: Timestamp = (10..MS_IN_YEAR).fake();
        assert_eq!(terms.get_interest(&account, &jar, time).0, time as u128);
    }
}

#[test]
fn register_score_based_product_with_signature() {
    let admin = admin();
    let mut context = Context::new(admin.clone());

    let signer = MessageSigner::new();
    let product = Product {
        id: "score_based_product".to_string(),
        cap: Cap::default(),
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: U64(MS_IN_YEAR),
            score_cap: 30_000,
        }),
        withdrawal_fee: None,
        public_key: Base64VecU8::from(signer.public_key()).into(),
        is_enabled: true,
    };

    context.switch_account(admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(product.clone()));

    assert_eq!(product.id, context.contract().get_products().first().unwrap().id);
}

#[test]
#[should_panic(expected = "Score based must be protected.")]
fn register_score_based_product_without_signature() {
    let admin = admin();
    let mut context = Context::new(admin.clone());

    let product_dto = Product {
        id: "score_based_product".to_string(),
        cap: Cap::default(),
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: U64(MS_IN_YEAR),
            score_cap: 30_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    context.switch_account(admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(product_dto.clone()));
}

#[test]
#[should_panic(expected = "Cap minimum must be less than maximum")]
fn register_product_with_inverted_cap() {
    let admin = admin();
    let mut context = Context::new(admin.clone());

    let product_dto = Product {
        id: "inverted_cap_product".to_string(),
        cap: Cap::new(1_000_000, 100),
        terms: Terms::Fixed(FixedProductTerms {
            lockup_term: U64(MS_IN_YEAR),
            apy: Apy::default(),
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    };

    context.switch_account(admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(product_dto.clone()));
}

fn generate_product() -> Product {
    Product::default().with_cap(100, 100_000_000_000)
}

fn register_product(product: Product) -> Product {
    let admin = admin();

    let mut context = Context::new(admin.clone());

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(product));
    let result = context.contract().get_products().first().unwrap().clone();

    result
}
