#![cfg(test)]

use std::collections::HashSet;

use itertools::Itertools;
use near_sdk::{AccountId, PromiseOrValue};
use rstest::{fixture, rstest};
use sweat_jar_model::{
    api::{AccountApi, ClaimApi, WithdrawApi},
    data::{
        jar::{Deposit, Jar},
        product::{Apy, FixedProductTerms, FlexibleProductTerms, Product, ProductId, Terms, WithdrawalFee},
        withdraw::BulkWithdrawView,
    },
    Timestamp, TokenAmount, UDecimal, MS_IN_DAY,
};

use crate::{
    common::{
        env::test_env_ext,
        testing::{accounts::*, expect_panic, Context, TokenUtils, UnwrapPromise},
    },
    feature::{
        account::model::test_utils::jar,
        product::model::test_utils::*,
        withdraw::api::{BulkWithdrawalRequest, WithdrawalDto, WithdrawalRequest},
    },
};

#[fixture]
fn product_fixed(#[default(365)] term_in_days: u64, #[default("product_fixed")] id: &str, product: Product) -> Product {
    product
        .with_id(id.to_string())
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: (term_in_days * MS_IN_DAY).into(),
            apy: Apy::Constant(UDecimal::new(12_000, 5)),
        }))
}

#[rstest]
fn withdraw_locked_jar_before_maturity_by_not_owner(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(365)]
    product: Product,
    #[with(vec![(0, 0)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.switch_account(context.owner.clone());
    expect_panic(&context, "Account owner is not found", || {
        context.contract().withdraw(product.id.clone());
    });

    assert_eq!(context.withdraw_all(&alice).total_amount.0, 0);
}

#[rstest]
fn withdraw_locked_jar_before_maturity_by_owner(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(200)]
    product: Product,
    #[with(vec![(100, 0)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_ms(120);

    context.switch_account(&alice);

    assert_eq!(0, context.withdraw(&alice, &product.id).withdrawn_amount.0);
    assert_eq!(0, context.withdraw_all(&alice).total_amount.0);
}

#[rstest]
fn withdraw_locked_jar_after_maturity_by_not_owner(
    admin: AccountId,
    alice: AccountId,
    #[values(365, 400, 500)] term_in_days: u64,
    #[from(product_fixed)]
    #[with(term_in_days)]
    product: Product,
    #[with(vec![(0, 0)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + 1);

    expect_panic(&context, "Account owner is not found", || {
        context.contract().withdraw(product.id);
    });

    assert_eq!(context.withdraw_all(&alice).total_amount.0, 0);
}

#[rstest]
fn withdraw_locked_jar_after_maturity_by_owner(
    admin: AccountId,
    alice: AccountId,
    #[values(365, 400, 500)] term_in_days: u64,
    #[from(product_fixed)]
    #[with(term_in_days)]
    product: Product,
    #[with(vec![(0, 0)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_ms(term_in_days * MS_IN_DAY + 1);

    assert_eq!(0, context.withdraw(&alice, &product.id).withdrawn_amount.0);
}

#[rstest]
#[should_panic(expected = "Account owner is not found")]
fn withdraw_flexible_jar_by_not_owner(
    admin: AccountId,
    alice: AccountId,
    #[from(product_flexible_10_percent)] product: Product,
    #[with(vec![(0, 0)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_days(1);
    context.contract().withdraw(product.id);
}

#[rstest]
fn withdraw_flexible_jar_by_owner_full(
    admin: AccountId,
    alice: AccountId,
    #[values(1_000_000, 1_000_000.to_otto(), 3.to_otto())] principal: TokenAmount,
    #[from(product_flexible_10_percent)] product: Product,
    #[with(vec![(0, principal)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_days(1);

    let withdrawn_amount = context.withdraw(&alice, &product.id);
    assert_eq!(principal, withdrawn_amount.withdrawn_amount.0);

    let interest = context.contract().get_total_interest(alice.clone());
    let claimed = context.contract().claim_total(None).unwrap();

    assert_ne!(0, claimed.get_total().0);
    assert_eq!(interest.amount.total, claimed.get_total());
    assert!(context.contract().get_jars_for_account(alice).is_empty());
}

#[rstest]
#[case(1_000_000, 200_000)]
#[case(5_000_000, 1_000_000)]
#[case(55_001, 11_000)]
fn dont_delete_jar_after_withdraw_with_interest_left(
    admin: AccountId,
    alice: AccountId,
    #[case] principal: TokenAmount,
    #[case] target_interest: TokenAmount,
    #[from(product_1_year_20_percent)] product: Product,
    #[with(vec![(0, principal)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + 1);

    let withdrawn = context.withdraw(&alice, &product.id);
    assert_eq!(withdrawn.withdrawn_amount.0, principal);
    assert_eq!(withdrawn.fee.0, 0);

    let jar = context.contract().get_account(&alice).get_jar(&product.id).clone();
    assert_eq!(jar.total_principal(), 0);
    assert_eq!(jar.cache.as_ref().unwrap().interest, target_interest);
}

#[rstest]
fn product_with_fixed_fee(
    admin: AccountId,
    alice: AccountId,
    #[values(10, 100, 500)] fee: TokenAmount,
    #[from(product_1_year_12_percent_with_fixed_fee)]
    #[with(fee)]
    product: Product,
    #[values(1_000_000, 1_000_000.to_otto(), 3.to_otto())] principal: TokenAmount,
    #[with(vec![(0, principal)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + 1);
    let withdraw = context.withdraw(&alice, &product.id);
    assert_eq!(withdraw.withdrawn_amount.0, principal - fee);
    assert_eq!(withdraw.fee.0, fee);
}

#[rstest]
fn text_product_with_percent_fee(
    admin: AccountId,
    alice: AccountId,
    #[values(UDecimal::new(5, 4), UDecimal::new(10_000, 5), UDecimal::new(1, 1))] fee: UDecimal,
    #[from(product_1_year_12_percent_with_percent_fee)]
    #[with(fee)]
    product: Product,
    #[values(1_000_000, 1_000_000.to_otto(), 3.to_otto())] principal: TokenAmount,
    #[with(vec![(0, principal)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + 1);
    let withdraw = context.withdraw(&alice, &product.id);
    let reference_fee = fee * principal;
    assert_eq!(withdraw.withdrawn_amount.0, principal - reference_fee);
    assert_eq!(withdraw.fee.0, reference_fee);
}

#[rstest]
fn test_failed_withdraw_promise(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(90)]
    product: Product,
    #[with(vec![(0, 1_000_000)])] jar: Jar,
) {
    test_env_ext::set_test_future_success(false);

    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar)]);

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + 1);
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

#[rstest]
fn test_failed_withdraw_internal(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(30)]
    product: Product,
    #[values(1_000_000, 3_000_000.to_otto(), 3.to_otto())] principal: TokenAmount,
    #[with(vec![(0, principal)])] jar: Jar,
) {
    let context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    let request = WithdrawalRequest {
        product_id: product.id.clone(),
        withdrawal: WithdrawalDto::new(jar.total_principal(), 0),
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

#[rstest]
fn test_failed_bulk_withdraw_internal(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(100)]
    product: Product,
    #[values(400_000, 7_000_000.to_otto(), 15.to_otto())] principal: TokenAmount,
    #[with(vec![(0, principal)])] jar: Jar,
) {
    let context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    let request = BulkWithdrawalRequest {
        requests: vec![WithdrawalRequest {
            product_id: product.id.clone(),
            withdrawal: WithdrawalDto::new(jar.total_principal(), 0),
            partition_index: 0,
        }],
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

#[rstest]
fn withdraw_from_locked_jar(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(10)]
    product: Product,
    #[with(vec![(0, 500_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context
        .contract()
        .get_account_mut(&alice)
        .get_jar_mut(&product.id)
        .lock();

    context.set_block_timestamp_in_ms(product.terms.get_lockup_term().unwrap() + 1);

    context.switch_account(&alice);
    expect_panic(&context, "Another operation on this Jar is in progress", || {
        context.contract().withdraw(product.id.clone());
    });

    assert!(context.withdraw_all(&alice).withdrawals.is_empty());
}

#[rstest]
fn withdraw_all(
    admin: AccountId,
    alice: AccountId,
    #[values(365, 730, 90)] test_duration_id_days: u64,
    #[from(product_fixed)]
    #[with(test_duration_id_days, "regular_product")]
    regular_product: Product,
    #[from(product_fixed)]
    #[with(test_duration_id_days * 2, "long_term_product")]
    long_term_product: Product,
    #[from(product_fixed)]
    #[with(90, "illegal_product")]
    illegal_product: Product,
    #[from(jar)]
    #[with(vec![(0, 10_000_000)])]
    regular_jar: Jar,
    #[from(jar)]
    #[with(vec![(0, 2_000_000)])]
    long_term_jar: Jar,
    #[from(jar)]
    #[with(vec![(0, 300_000)])]
    mut illegal_jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[
            regular_product.clone(),
            long_term_product.clone(),
            illegal_product.clone(),
        ])
        .with_jars(
            &alice,
            &[
                (regular_product.id, regular_jar.clone()),
                (long_term_product.id, long_term_jar.clone()),
                (illegal_product.id, illegal_jar.lock().clone()),
            ],
        );

    context.set_block_timestamp_in_days(test_duration_id_days + 1);

    context.switch_account(alice.clone());
    context.contract().claim_total(None);

    let withdrawn = context.withdraw_all(&alice);
    assert_eq!(regular_jar.total_principal(), withdrawn.total_amount.0);

    let jars = context.contract().get_jars_for_account(alice.clone());
    let jars_principal: Vec<TokenAmount> = jars.into_iter().map(|j| j.principal.0).sorted().collect();
    let target_principal: Vec<TokenAmount> = [illegal_jar.total_principal(), long_term_jar.total_principal()]
        .iter()
        .sorted()
        .cloned()
        .collect();
    assert_eq!(jars_principal, target_principal);
}

#[rstest]
#[case(100, UDecimal::new(1, 2))]
#[case(200, UDecimal::new(2, 2))]
fn withdraw_all_with_fee(
    admin: AccountId,
    alice: AccountId,
    #[case] fixed_fee: TokenAmount,
    #[from(product_1_year_12_percent_with_fixed_fee)]
    #[with(fixed_fee)]
    product_with_fixed_fee: Product,
    #[case] percent_fee: UDecimal,
    #[from(product_1_year_12_percent_with_percent_fee)]
    #[with(percent_fee)]
    product_with_percent_fee: Product,
    #[from(jar)]
    #[with(vec![(0, 100.to_otto())])]
    jar_with_fixed_fee: Jar,
    #[from(jar)]
    #[with(vec![(0, 1_000.to_otto())])]
    jar_with_percent_fee: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product_with_fixed_fee.clone(), product_with_percent_fee.clone()])
        .with_jars(
            &alice,
            &[
                (product_with_fixed_fee.id, jar_with_fixed_fee.clone()),
                (product_with_percent_fee.id, jar_with_percent_fee.clone()),
            ],
        );

    context.set_block_timestamp_in_days(product_with_fixed_fee.terms.get_lockup_term().unwrap() + 1);
    context.switch_account(alice.clone());
    context.contract().claim_total(None);

    let withdrawn = context.withdraw_all(&alice);
    let total_fee = withdrawn.withdrawals.iter().map(|withdrawal| withdrawal.fee.0).sum();
    let expected_fee = fixed_fee + percent_fee * jar_with_percent_fee.total_principal();
    assert_eq!(expected_fee, total_fee);
    assert_eq!(expected_fee, context.contract().fee_amount);
}

#[rstest]
fn batch_withdraw_all(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(180)]
    product: Product,
    #[from(jar)]
    #[with(vec![(0, 7_000_000), (MS_IN_DAY, 300_000), (2 * MS_IN_DAY, 20_000)])]
    jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id, jar.clone())]);

    // One day after last deposit unlock
    context.set_block_timestamp_in_ms(
        product.terms.get_lockup_term().unwrap() + jar.deposits.last().unwrap().created_at + MS_IN_DAY,
    );

    context.switch_account(alice.clone());
    context.contract().claim_total(None);
    let withdrawn = context.withdraw_all(&alice);

    let withdrawn_amount = withdrawn.withdrawals.first().unwrap().withdrawn_amount.0;
    let total_deposits_principal = jar
        .deposits
        .into_iter()
        .map(|deposit| deposit.principal)
        .sum::<TokenAmount>();
    assert_eq!(total_deposits_principal, withdrawn_amount);

    let jars = context.contract().get_jars_for_account(alice.clone());
    assert!(jars.is_empty());
}

#[rstest]
fn batch_withdraw_partially(
    admin: AccountId,
    alice: AccountId,
    #[from(product_fixed)]
    #[with(180, "product_1")]
    product_1: Product,
    #[from(product_fixed)]
    #[with(180, "product_2")]
    product_2: Product,
    #[from(product_fixed)]
    #[with(180, "product_3")]
    product_3: Product,
    #[from(jar)]
    #[with(vec![(0, 7_000_000), (MS_IN_DAY, 300_000), (2 * MS_IN_DAY, 20_000)])]
    jar_1: Jar,
    #[from(jar)]
    #[with(vec![(0, 1_000_000), (MS_IN_DAY, 400_000)])]
    jar_2: Jar,
    #[from(jar)]
    #[with(vec![(0, 17_000_000)])]
    jar_3: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product_1.clone(), product_2.clone(), product_3.clone()])
        .with_jars(
            &alice,
            &[
                (product_1.id.clone(), jar_1.clone()),
                (product_2.id.clone(), jar_2.clone()),
                (product_3.id.clone(), jar_3.clone()),
            ],
        );

    // One day after last deposit unlock
    context.set_block_timestamp_in_ms(
        product_1.terms.get_lockup_term().unwrap() + jar_1.deposits.last().unwrap().created_at + MS_IN_DAY,
    );

    context.switch_account(alice.clone());
    context.contract().claim_total(None);
    let withdrawn = context.withdraw_bulk(&alice, HashSet::from([product_1.id.clone(), product_2.id.clone()]));

    let total_target_deposits_principal = [jar_1.deposits, jar_2.deposits]
        .concat()
        .into_iter()
        .map(|deposit| deposit.principal)
        .sum::<TokenAmount>();
    assert_eq!(total_target_deposits_principal, withdrawn.total_amount.0);

    let jars = context.contract().get_jars_for_account(alice.clone());
    assert_eq!(1, jars.len());
    assert_eq!(
        jar_3.deposits.first().unwrap().principal,
        jars.first().unwrap().principal.0
    );
}

impl Context {
    fn withdraw_all(&mut self, account_id: &AccountId) -> BulkWithdrawView {
        self.withdraw_internal(account_id, None)
    }

    fn withdraw_bulk(&mut self, account_id: &AccountId, product_ids: HashSet<ProductId>) -> BulkWithdrawView {
        self.withdraw_internal(account_id, product_ids.into())
    }

    fn withdraw_internal(
        &mut self,
        account_id: &AccountId,
        product_ids: Option<HashSet<ProductId>>,
    ) -> BulkWithdrawView {
        self.switch_account(account_id);
        let result = self.contract().withdraw_all(product_ids);

        match result {
            PromiseOrValue::Promise(_) => {
                panic!("Expected value");
            }
            PromiseOrValue::Value(value) => value,
        }
    }
}
