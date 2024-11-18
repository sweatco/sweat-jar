#![cfg(test)]

use itertools::Itertools;
use near_sdk::{test_utils::test_env::alice, AccountId, PromiseOrValue};
use sweat_jar_model::{
    api::{ClaimApi, JarApi, WithdrawApi},
    withdraw::BulkWithdrawView,
    TokenAmount, UDecimal, MS_IN_DAY,
};

use crate::{
    common::{test_data::set_test_future_success, tests::Context, Timestamp},
    jar::model::{Deposit, JarV2},
    product::model::{Apy, Cap, FixedProductTerms, FlexibleProductTerms, ProductV2, Terms, WithdrawalFee},
    test_utils::{admin, expect_panic, UnwrapPromise},
    withdraw::api::{BulkWithdrawalRequest, WithdrawalRequest},
};

fn testing_product_fixed(term_in_days: u64) -> ProductV2 {
    let term_in_ms = term_in_days * MS_IN_DAY;

    ProductV2 {
        id: "regular_product".to_string(),
        cap: Cap {
            min: 0,
            max: 1_000_000_000_000_000,
        },
        terms: Terms::Fixed(FixedProductTerms {
            lockup_term: term_in_ms,
            apy: Apy::Constant(UDecimal::new(12000, 5)),
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    }
}

fn testing_product_flexible() -> ProductV2 {
    ProductV2 {
        id: "flexible_product".to_string(),
        cap: Cap {
            min: 0,
            max: 1_000_000_000_000_000,
        },
        terms: Terms::Flexible(FlexibleProductTerms {
            apy: Apy::Constant(UDecimal::new(12000, 5)),
        }),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: true,
    }
}

fn prepare_jar(product: &ProductV2) -> (AccountId, JarV2, Context) {
    prepare_jar_with_deposit(product, None, None)
}

fn prepare_jar_with_deposit(
    product: &ProductV2,
    created_at: Option<Timestamp>,
    principal: Option<TokenAmount>,
) -> (AccountId, JarV2, Context) {
    let jar = JarV2::new().with_deposit(created_at.unwrap_or_default(), principal.unwrap_or_default());

    let context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), jar.clone())]);

    (alice().clone(), jar, context)
}

#[test]
fn withdraw_locked_jar_before_maturity_by_not_owner() {
    let product = testing_product_fixed(365);
    let (_, _, mut context) = prepare_jar(&product);

    expect_panic(&context, "Account owner is not found", || {
        context.contract().withdraw(product.id.clone());
    });

    assert_eq!(context.withdraw_all(&alice()).total_amount.0, 0);
}

#[test]
fn withdraw_locked_jar_before_maturity_by_owner() {
    let product = testing_product_fixed(200);
    let (alice, jar, mut context) = prepare_jar_with_deposit(&product, Some(100), None);

    context.set_block_timestamp_in_ms(120);

    context.switch_account(&alice);

    assert_eq!(0, context.withdraw(&alice, &product.id).withdrawn_amount.0);
    assert_eq!(0, context.withdraw_all(&alice).total_amount.0);
}

#[test]
fn withdraw_locked_jar_after_maturity_by_not_owner() {
    let term_in_days = 365;
    let product = testing_product_fixed(term_in_days);
    let (_, _, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + 1);

    expect_panic(&context, "Account owner is not found", || {
        context.contract().withdraw(product.id);
    });

    assert_eq!(context.withdraw_all(&alice()).total_amount.0, 0);
}

#[test]
fn withdraw_locked_jar_after_maturity_by_owner() {
    let term_in_days = 365;
    let product = testing_product_fixed(term_in_days);
    let (alice, _, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + 1);

    assert_eq!(0, context.withdraw(&alice, &product.id).withdrawn_amount.0);
}

#[test]
#[should_panic(expected = "Account owner is not found")]
fn withdraw_flexible_jar_by_not_owner() {
    let product = testing_product_flexible();
    let (_, jar, mut context) = prepare_jar(&product);

    context.set_block_timestamp_in_days(1);
    context.contract().withdraw(product.id);
}

#[test]
fn withdraw_flexible_jar_by_owner_full() {
    let product = testing_product_flexible();
    let principal = 1_000_000;
    let (alice, reference_jar, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(principal));

    context.set_block_timestamp_in_days(1);

    let withdrawn_amount = context.withdraw(&alice, &product.id);
    assert_eq!(principal, withdrawn_amount.withdrawn_amount.0);

    let interest = context.contract().get_total_interest(alice.clone());
    let claimed = context.contract().claim_total(None).unwrap();

    assert_ne!(0, claimed.get_total().0);
    assert_eq!(interest.amount.total, claimed.get_total());
    assert!(context.contract().get_jars_for_account(alice).is_empty());
}

#[test]
fn dont_delete_jar_after_withdraw_with_interest_left() {
    let term_in_days = 365;
    let principal = 1_000_000;
    let target_interest = 200_000;
    let product = testing_product_fixed(term_in_days).with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: term_in_days * MS_IN_DAY,
        apy: Apy::Constant(UDecimal::new(20000, 5)),
    }));
    let (alice, _, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(principal));

    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + 1);

    let withdrawn = context.withdraw(&alice, &product.id);
    assert_eq!(withdrawn.withdrawn_amount.0, principal);
    assert_eq!(withdrawn.fee.0, 0);

    let jar = context.contract().get_account(&alice).get_jar(&product.id).clone();
    assert_eq!(jar.total_principal(), 0);
    assert_eq!(jar.cache.as_ref().unwrap().interest, target_interest);
}

#[test]
fn product_with_fixed_fee() {
    let term_in_days = 365;
    let principal = 1_000_000;
    let fee = 10;
    let product = testing_product_fixed(term_in_days)
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: term_in_days * MS_IN_DAY,
            apy: Apy::Constant(UDecimal::new(20000, 5)),
        }))
        .with_withdrawal_fee(WithdrawalFee::Fix(fee));
    let (alice, _, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(principal));

    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + 1);
    let withdraw = context.withdraw(&alice, &product.id);
    assert_eq!(withdraw.withdrawn_amount.0, principal - fee);
    assert_eq!(withdraw.fee.0, fee);
}

#[test]
fn product_with_percent_fee() {
    let term_in_days = 365;
    let principal = 1_000_000;
    let fee = UDecimal::new(5, 4);
    let product = testing_product_fixed(term_in_days)
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: term_in_days * MS_IN_DAY,
            apy: Apy::Constant(UDecimal::new(20000, 5)),
        }))
        .with_withdrawal_fee(WithdrawalFee::Percent(fee.clone()));
    let (alice, _, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(principal));

    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + 1);
    let withdraw = context.withdraw(&alice, &product.id);
    let reference_fee = fee * principal;
    assert_eq!(withdraw.withdrawn_amount.0, principal - reference_fee);
    assert_eq!(withdraw.fee.0, reference_fee);
}

#[test]
fn test_failed_withdraw_promise() {
    set_test_future_success(false);

    let term_id_days = 90;
    let product = testing_product_fixed(term_id_days);
    let (alice, jar, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(1_000_000));

    context.set_block_timestamp_in_ms(term_id_days * MS_IN_DAY + 1);
    context.switch_account(&alice);

    let total_principal_before_withdrawal = context
        .contract()
        .get_account(&alice)
        .get_jar(&product.id)
        .total_principal();

    let withdrawn = context.withdraw(&alice, &product.id);
    assert_eq!(withdrawn.withdrawn_amount.0, 0);

    let total_principal_after_withdrawal = context
        .contract()
        .get_account(&alice)
        .get_jar(&product.id)
        .total_principal();
    assert_eq!(total_principal_before_withdrawal, total_principal_after_withdrawal);
}

#[test]
fn test_failed_withdraw_internal() {
    let term_id_days = 30;
    let principal = 3_000_000;
    let product = testing_product_fixed(term_id_days);
    let (alice, jar, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(principal));

    let request = WithdrawalRequest {
        product_id: product.id.clone(),
        amount: jar.total_principal(),
        fee: 0,
        partition_index: 0,
    };
    let withdraw = context
        .contract()
        .after_withdraw_internal(alice.clone(), request, false);

    assert_eq!(withdraw.withdrawn_amount.0, 0);
    assert_eq!(withdraw.fee.0, 0);

    let current_principal = context
        .contract()
        .get_account(&alice)
        .get_jar(&product.id)
        .total_principal();
    assert_eq!(principal, current_principal);
}

#[test]
fn test_failed_bulk_withdraw_internal() {
    let term_id_days = 100;
    let principal = 400_000;
    let product = testing_product_fixed(term_id_days);
    let (alice, jar, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(principal));

    let request = BulkWithdrawalRequest {
        requests: vec![WithdrawalRequest {
            product_id: product.id.clone(),
            amount: jar.total_principal(),
            fee: 0,
            partition_index: 0,
        }],
        total_amount: jar.total_principal(),
        total_fee: 0,
    };

    let withdraw = context
        .contract()
        .after_bulk_withdraw_internal(alice.clone(), request, false);

    assert!(withdraw.withdrawals.is_empty());
    assert_eq!(withdraw.total_amount.0, 0);

    let current_principal = context
        .contract()
        .get_account(&alice)
        .get_jar(&product.id)
        .total_principal();
    assert_eq!(principal, current_principal);
}

#[test]
fn withdraw_from_locked_jar() {
    let term_id_days = 10;
    let principal = 500_000;
    let product = testing_product_fixed(term_id_days);
    let (alice, jar, mut context) = prepare_jar_with_deposit(&product, Some(0), Some(principal));

    context
        .contract()
        .get_account_mut(&alice)
        .get_jar_mut(&product.id)
        .lock();

    context.set_block_timestamp_in_ms(term_id_days * MS_IN_DAY + 1);

    context.switch_account(&alice);
    expect_panic(&context, "Another operation on this Jar is in progress", || {
        context.contract().withdraw(product.id.clone());
    });

    assert!(context.withdraw_all(&alice).withdrawals.is_empty());
}

#[test]
fn withdraw_all() {
    let test_duration_id_days = 365;

    let regular_product = testing_product_fixed(test_duration_id_days).id("regular_product");
    let long_term_product = testing_product_fixed(test_duration_id_days * 2).id("long_term_product");
    let illegal_product = testing_product_fixed(90).id("illegal_product");

    let regular_principal = 10_000_000;
    let long_term_principal = 2_000_000;
    let illegal_principal = 300_000;

    let mut context = Context::new(admin())
        .with_products(&[
            regular_product.clone(),
            long_term_product.clone(),
            illegal_product.clone(),
        ])
        .with_jars(
            &alice(),
            &[
                (regular_product.id, JarV2::new().with_deposit(0, regular_principal)),
                (long_term_product.id, JarV2::new().with_deposit(0, long_term_principal)),
                (
                    illegal_product.id,
                    JarV2::new().with_deposit(0, illegal_principal).lock().clone(),
                ),
            ],
        );

    context.set_block_timestamp_in_days(test_duration_id_days + 1);

    context.switch_account(&alice());
    context.contract().claim_total(None);

    let withdrawn = context.withdraw_all(&alice());
    assert_eq!(regular_principal, withdrawn.total_amount.0);

    let jars = context.contract().get_jars_for_account(alice());
    let jars_principal: Vec<TokenAmount> = jars.into_iter().map(|j| j.principal.0).sorted().collect();
    let target_principal: Vec<TokenAmount> = [illegal_principal, long_term_principal]
        .iter()
        .sorted()
        .cloned()
        .collect();
    assert_eq!(jars_principal, target_principal);
}

#[test]
fn batch_withdraw_all() {
    let term_in_days = 180;
    let product = testing_product_fixed(term_in_days);
    let deposits = [(0, 7_000_000), (MS_IN_DAY, 300_000), (2 * MS_IN_DAY, 20_000)];
    let jar = JarV2 {
        deposits: deposits
            .into_iter()
            .map(|(created_at, principal)| Deposit::new(created_at, principal))
            .collect(),
        ..JarV2::new()
    };
    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id, jar)]);

    // One day after last deposit unlock
    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + deposits.last().unwrap().0 + MS_IN_DAY);

    context.switch_account(&alice());
    context.contract().claim_total(None);
    let withdrawn = context.withdraw_all(&alice());

    let withdrawn_amount = withdrawn.withdrawals.first().unwrap().withdrawn_amount.0;
    let total_deposits_principal = deposits
        .into_iter()
        .map(|(_, principal)| principal)
        .sum::<TokenAmount>();
    assert_eq!(total_deposits_principal, withdrawn_amount);

    let jars = context.contract().get_jars_for_account(alice());
    assert!(jars.is_empty());
}

impl Context {
    fn withdraw_all(&mut self, account_id: &AccountId) -> BulkWithdrawView {
        self.switch_account(account_id);
        let result = self.contract().withdraw_all();

        match result {
            PromiseOrValue::Promise(_) => {
                panic!("Expected value");
            }
            PromiseOrValue::Value(value) => value,
        }
    }
}
