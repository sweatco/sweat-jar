#![cfg(test)]

use fake::Fake;
use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    test_utils::test_env::alice,
};
use sweat_jar_model::{
    api::ProductApi,
    product::{
        ApyDto, ApyView, DowngradableApyView, FixedProductTermsDto, FlexibleProductTermsDto, ProductDto, ProductView,
        ScoreBasedProductTermsDto, TermsDto, TermsView, WithdrawalFeeDto, WithdrawalFeeView,
    },
    UDecimal, MS_IN_YEAR,
};

use crate::{
    common::{tests::Context, Timestamp},
    jar::{account::Account, model::Jar},
    product::{
        helpers::MessageSigner,
        model::{Apy, DowngradableApy, FixedProductTerms, InterestCalculator, Product, Terms, WithdrawalFee},
    },
    test_utils::admin,
};

pub(crate) fn get_product_dto() -> ProductDto {
    ProductDto {
        id: "product".to_string(),
        ..Default::default()
    }
}

#[test]
fn disable_product_when_enabled() {
    let admin = admin();
    let product = &Product::new();

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
    let product = &Product::new();

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
        let first_command = get_product_dto();
        context.contract().register_product(first_command)
    });

    context.with_deposit_yocto(1, |context| {
        let second_command = get_product_dto();
        context.contract().register_product(second_command)
    });
}

#[test]
fn register_downgradable_product() {
    let (product, view) = register_product(ProductDto {
        id: "downgradable_product".to_string(),
        terms: TermsDto::Fixed(FixedProductTermsDto {
            apy: ApyDto {
                fallback: Some((U128(10), 3)),
                ..ApyDto::default()
            },
            lockup_term: Default::default(),
        }),
        ..Default::default()
    });

    assert_eq!(
        product.get_base_apy().clone(),
        Apy::Downgradable(DowngradableApy {
            default: UDecimal {
                significand: 12,
                exponent: 2
            },
            fallback: UDecimal {
                significand: 10,
                exponent: 3
            },
        })
    );

    assert_eq!(
        view.get_base_apy().clone(),
        ApyView::Downgradable(DowngradableApyView {
            default: 0.12,
            fallback: 0.01
        })
    )
}

#[test]
#[should_panic(
    expected = "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
)]
fn register_product_with_too_high_fixed_fee() {
    register_product(ProductDto {
        id: "product_with_fixed_fee".to_string(),
        withdrawal_fee: WithdrawalFeeDto::Fix(U128(200)).into(),
        terms: TermsDto::Fixed(FixedProductTermsDto {
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
    register_product(ProductDto {
        id: "product_with_fixed_fee".to_string(),
        withdrawal_fee: WithdrawalFeeDto::Percent(U128(100), 0).into(),
        ..Default::default()
    });
}

#[test]
fn register_product_with_fee() {
    let (product, view) = register_product(ProductDto {
        id: "product_with_fixed_fee".to_string(),
        withdrawal_fee: WithdrawalFeeDto::Fix(U128(10)).into(),
        ..Default::default()
    });

    assert_eq!(product.withdrawal_fee, Some(WithdrawalFee::Fix(10)));

    assert_eq!(view.withdrawal_fee, Some(WithdrawalFeeView::Fix(U128(10))));

    let (product, view) = register_product(ProductDto {
        id: "product_with_percent_fee".to_string(),
        withdrawal_fee: WithdrawalFeeDto::Percent(U128(12), 2).into(),
        ..Default::default()
    });

    assert_eq!(
        product.withdrawal_fee,
        Some(WithdrawalFee::Percent(UDecimal {
            significand: 12,
            exponent: 2
        }))
    );

    assert_eq!(view.withdrawal_fee, Some(WithdrawalFeeView::Percent(0.12)));
}

#[test]
fn register_product_with_flexible_terms() {
    let (product, view) = register_product(ProductDto {
        id: "product_with_fixed_fee".to_string(),
        terms: TermsDto::Flexible(FlexibleProductTermsDto { apy: ApyDto::default() }),
        ..ProductDto::default()
    });

    assert!(matches!(product.terms, Terms::Flexible(_)));
    assert!(matches!(view.terms, TermsView::Flexible(_)));
}

#[test]
fn set_public_key() {
    let admin = admin();

    let signer = MessageSigner::new();
    let product = generate_product().public_key(signer.public_key());
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
    assert_eq!(&new_pk, product.public_key.as_ref().unwrap());
}

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn set_public_key_by_not_admin() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = generate_product().public_key(signer.public_key());
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
    let product = generate_product().public_key(signer.public_key());
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
        lockup_term: 2 * MS_IN_YEAR,
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
        lockup_term: MS_IN_YEAR,
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
        lockup_term: MS_IN_YEAR,
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
    let product_dto = ProductDto {
        id: "score_based_product".to_string(),
        cap: (0.into(), 10_000_000.into()),
        terms: TermsDto::ScoreBased(ScoreBasedProductTermsDto {
            lockup_term: U64(MS_IN_YEAR),
            score_cap: 30_000,
        }),
        withdrawal_fee: None,
        public_key: Base64VecU8::from(signer.public_key()).into(),
        is_enabled: true,
        is_restakable: true,
    };

    context.switch_account(admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(product_dto.clone()));

    assert_eq!(product_dto.id, context.contract().get_products().first().unwrap().id);
}

#[test]
#[should_panic(expected = "Score based must be protected.")]
fn register_score_based_product_without_signature() {
    let admin = admin();
    let mut context = Context::new(admin.clone());

    let product_dto = ProductDto {
        id: "score_based_product".to_string(),
        cap: (0.into(), 10_000_000.into()),
        terms: TermsDto::ScoreBased(ScoreBasedProductTermsDto {
            lockup_term: U64(MS_IN_YEAR),
            score_cap: 30_000,
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
        is_restakable: true,
    };

    context.switch_account(admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(product_dto.clone()));
}

fn generate_product() -> Product {
    Product::new().cap(100, 100_000_000_000)
}

fn register_product(command: ProductDto) -> (Product, ProductView) {
    let admin = admin();

    let mut context = Context::new(admin.clone());

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(command));

    let product = context.contract().products.into_iter().last().unwrap().1.clone();
    let view = context.contract().get_products().first().unwrap().clone();

    (product, view)
}

impl Product {
    fn get_base_apy(&self) -> &Apy {
        match &self.terms {
            Terms::Fixed(value) => &value.apy,
            Terms::Flexible(value) => &value.apy,
            Terms::ScoreBased(_) => panic!("No APY for a score based product"),
        }
    }
}
