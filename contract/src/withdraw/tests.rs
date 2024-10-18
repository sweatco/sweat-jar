// #![cfg(test)]
//
// use near_sdk::{json_types::U128, test_utils::test_env::alice, AccountId};
// use sweat_jar_model::{
//     api::{ClaimApi, JarApi, WithdrawApi},
//     UDecimal, MS_IN_YEAR, U32,
// };
//
// use crate::{
//     common::{test_data::set_test_future_success, tests::Context, Timestamp},
//     jar::model::Jar,
//     product::model::{Apy, Product, WithdrawalFee},
//     test_utils::{admin, expect_panic, UnwrapPromise, PRINCIPAL},
//     withdraw::api::WithdrawalRequest,
// };
//
// fn prepare_jar(product: &Product) -> (AccountId, Jar, Context) {
//     let alice = alice();
//     let admin = admin();
//
//     let jar = Jar::new(0);
//     let context = Context::new(admin)
//         .with_products(&[product.clone()])
//         .with_jars(&[jar.clone()]);
//
//     (alice, jar, context)
// }
//
// fn prepare_jar_created_at(product: &Product, created_at: Timestamp) -> (AccountId, Jar, Context) {
//     let alice = alice();
//     let admin = admin();
//
//     let jar = Jar::new(0).created_at(created_at);
//     let context = Context::new(admin)
//         .with_products(&[product.clone()])
//         .with_jars(&[jar.clone()]);
//
//     (alice, jar, context)
// }
//
// #[test]
// fn withdraw_locked_jar_before_maturity_by_not_owner() {
//     let (_, _, context) = prepare_jar(&Product::new());
//
//     expect_panic(&context, "Account 'owner' doesn't exist", || {
//         context.contract().withdraw(U32(0), None);
//     });
//
//     assert_eq!(context.contract().withdraw_all(None).unwrap().total_amount.0, 0);
// }
//
// #[test]
// fn withdraw_locked_jar_before_maturity_by_owner() {
//     let (alice, jar, mut context) = prepare_jar_created_at(&Product::new().lockup_term(200), 100);
//
//     context.set_block_timestamp_in_ms(120);
//
//     context.switch_account(&alice);
//
//     expect_panic(&context, "The jar is not mature yet", || {
//         context.contract().withdraw(U32(jar.id), None);
//     });
//
//     assert!(context.contract().withdraw_all(None).unwrap().jars.is_empty());
// }
//
// #[test]
// fn withdraw_locked_jar_after_maturity_by_not_owner() {
//     let product = Product::new();
//     let (_, jar, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
//
//     expect_panic(&context, "Account 'owner' doesn't exist", || {
//         context.contract().withdraw(U32(jar.id), None);
//     });
//
//     assert_eq!(context.contract().withdraw_all(None).unwrap().total_amount.0, 0);
// }
//
// #[test]
// fn withdraw_locked_jar_after_maturity_by_owner() {
//     let product = Product::new();
//     let (alice, jar, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
//     context.switch_account(&alice);
//     context.contract().withdraw(U32(jar.id), None);
// }
//
// #[test]
// #[should_panic(expected = "Account 'owner' doesn't exist")]
// fn withdraw_flexible_jar_by_not_owner() {
//     let product = Product::new().flexible();
//     let (_, jar, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_days(1);
//     context.contract().withdraw(U32(jar.id), None);
// }
//
// #[test]
// fn withdraw_flexible_jar_by_owner_full() {
//     let product = Product::new().flexible();
//     let (alice, reference_jar, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_days(1);
//     context.switch_account(&alice);
//
//     context.contract().withdraw(U32(reference_jar.id), None);
//
//     let interest = context
//         .contract()
//         .get_interest(vec![reference_jar.id.into()], alice.clone());
//
//     let claimed = context.contract().claim_total(None).unwrap();
//
//     assert_eq!(interest.amount.total, claimed.get_total());
//
//     let jar = context.contract().get_jar(alice.clone(), U32(reference_jar.id));
//     assert_eq!(0, jar.principal.0);
// }
//
// #[test]
// fn withdraw_flexible_jar_by_owner_with_sufficient_balance() {
//     let product = Product::new().flexible();
//     let (alice, reference_jar, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_days(1);
//     context.switch_account(&alice);
//
//     context.contract().withdraw(U32(0), Some(U128(100_000)));
//     let jar = context.contract().get_jar(alice.clone(), U32(reference_jar.id));
//     assert_eq!(900_000, jar.principal.0);
// }
//
// #[test]
// fn withdraw_flexible_jar_by_owner_with_insufficient_balance() {
//     let product = Product::new().flexible();
//     let (alice, jar, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_days(1);
//     context.switch_account(&alice);
//
//     expect_panic(&context, "Insufficient balance", || {
//         context.contract().withdraw(U32(jar.id), Some(U128(2_000_000)));
//     });
//
//     let withdrawn = context.contract().withdraw_all(None).unwrap();
//
//     assert_eq!(withdrawn.jars.len(), 1);
//     assert_eq!(withdrawn.jars[0].withdrawn_amount.0, 1_000_000);
// }
//
// #[test]
// fn dont_delete_jar_after_withdraw_with_interest_left() {
//     let product = Product::new().apy(Apy::Constant(UDecimal::new(2, 1)));
//
//     let (alice, _, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
//     context.switch_account(&alice);
//
//     let jar = context.contract().get_jar_internal(&alice, 0);
//
//     let withdrawn = context.contract().withdraw(U32(jar.id), Some(U128(1_000_000))).unwrap();
//
//     assert_eq!(withdrawn.withdrawn_amount, U128(1_000_000));
//     assert_eq!(withdrawn.fee, U128(0));
//
//     let jar = context.contract().get_jar_internal(&alice, 0);
//     assert_eq!(jar.principal, 0);
//
//     assert_eq!(jar.cache.as_ref().unwrap().interest, 200_000);
// }
//
// #[test]
// fn product_with_fixed_fee() {
//     let fee = 10;
//     let product = Product::new().with_withdrawal_fee(WithdrawalFee::Fix(fee));
//     let (alice, reference_jar, mut context) = prepare_jar(&product);
//
//     let initial_principal = reference_jar.principal;
//
//     context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
//     context.switch_account(&alice);
//
//     let withdraw_amount = 100_000;
//     let withdraw = context
//         .contract()
//         .withdraw(U32(0), Some(U128(withdraw_amount)))
//         .unwrap();
//
//     assert_eq!(withdraw.withdrawn_amount, U128(withdraw_amount - fee));
//     assert_eq!(withdraw.fee, U128(fee));
//
//     let jar = context.contract().get_jar(alice, U32(reference_jar.id));
//
//     assert_eq!(jar.principal, U128(initial_principal - withdraw_amount));
// }
//
// #[test]
// fn product_with_percent_fee() {
//     let fee_value = UDecimal::new(5, 4);
//     let fee = WithdrawalFee::Percent(fee_value.clone());
//     let product = Product::new().with_withdrawal_fee(fee);
//     let (alice, reference_jar, mut context) = prepare_jar(&product);
//
//     let initial_principal = reference_jar.principal;
//
//     context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
//     context.switch_account(&alice);
//
//     let withdrawn_amount = 100_000;
//     let withdraw = context
//         .contract()
//         .withdraw(U32(0), Some(U128(withdrawn_amount)))
//         .unwrap();
//
//     let reference_fee = fee_value * initial_principal;
//     assert_eq!(withdraw.withdrawn_amount, U128(withdrawn_amount - reference_fee));
//     assert_eq!(withdraw.fee, U128(reference_fee));
//
//     let jar = context.contract().get_jar(alice, U32(reference_jar.id));
//
//     assert_eq!(jar.principal, U128(initial_principal - withdrawn_amount));
// }
//
// #[test]
// fn test_failed_withdraw_promise() {
//     set_test_future_success(false);
//
//     let product = Product::new();
//     let (alice, reference_jar, mut context) = prepare_jar(&product);
//
//     context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
//     context.switch_account(&alice);
//
//     let jar_before_withdrawal = context.contract().get_jar(alice.clone(), U32(reference_jar.id));
//
//     let withdrawn = context.contract().withdraw(U32(0), Some(U128(100_000))).unwrap();
//
//     assert_eq!(withdrawn.withdrawn_amount.0, 0);
//
//     let jar_after_withdrawal = context.contract().get_jar(alice.clone(), U32(reference_jar.id));
//
//     assert_eq!(jar_before_withdrawal, jar_after_withdrawal);
// }
//
// #[test]
// fn test_failed_withdraw_internal() {
//     let product = Product::new();
//     let (alice, reference_jar, context) = prepare_jar(&product);
//     let withdrawn_amount = 1_234;
//
//     let mut contract = context.contract();
//
//     let jar_view = contract.get_jar(alice.clone(), U32(reference_jar.id));
//     let jar = contract.accounts.get(&alice).unwrap().iter().next().unwrap().clone();
//
//     let withdraw =
//         contract.after_withdraw_internal(jar.account_id.clone(), jar.id, true, withdrawn_amount, None, false);
//
//     assert_eq!(withdraw.withdrawn_amount, U128(0));
//     assert_eq!(withdraw.fee, U128(0));
//
//     assert_eq!(
//         jar_view.principal.0 + withdrawn_amount,
//         contract.get_jar(alice, U32(0)).principal.0
//     );
// }
//
// #[test]
// fn test_failed_bulk_withdraw_internal() {
//     let product = Product::new();
//     let (alice, reference_jar, context) = prepare_jar(&product);
//
//     let mut contract = context.contract();
//
//     let jar_view = contract.get_jar(alice.clone(), U32(reference_jar.id));
//     let jar = contract.accounts.get(&alice).unwrap().iter().next().unwrap().clone();
//
//     let withdraw = contract.after_bulk_withdraw_internal(
//         jar.account_id.clone(),
//         vec![WithdrawalRequest {
//             jar: jar.clone(),
//             should_be_closed: true,
//             amount: jar.principal,
//             fee: None,
//         }],
//         false,
//     );
//
//     assert!(withdraw.jars.is_empty());
//     assert_eq!(withdraw.total_amount.0, 0);
//
//     assert_eq!(
//         jar_view.principal.0 + jar_view.principal.0,
//         contract.get_jar(alice, U32(0)).principal.0
//     );
// }
//
// #[test]
// fn withdraw_from_locked_jar() {
//     let product = Product::new().apy(Apy::Constant(UDecimal::new(1, 0)));
//     let mut jar = Jar::new(0).principal(MS_IN_YEAR as u128);
//
//     jar.lock();
//
//     let alice = alice();
//     let admin = admin();
//
//     let mut context = Context::new(admin).with_products(&[product.clone()]).with_jars(&[jar]);
//
//     context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
//     context.switch_account(&alice);
//
//     expect_panic(&context, "Another operation on this Jar is in progress", || {
//         _ = context.contract().withdraw(U32(0), Some(U128(100_000)));
//     });
//
//     assert!(context.contract().withdraw_all(None).unwrap().jars.is_empty());
// }
//
// #[test]
// fn withdraw_all() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new();
//     let long_term_product = Product::new().id("long_term_product").lockup_term(MS_IN_YEAR * 2);
//
//     let mature_jar = Jar::new(1).principal(PRINCIPAL + 1);
//
//     let immature_jar = Jar::new(2).product_id(&long_term_product.id).principal(PRINCIPAL + 3);
//     let locked_jar = Jar::new(3).product_id(&product.id).principal(PRINCIPAL + 4).locked();
//
//     let mut context = Context::new(admin)
//         .with_products(&[product, long_term_product])
//         .with_jars(&[mature_jar.clone(), immature_jar.clone(), locked_jar.clone()]);
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//
//     context.contract().claim_total(None);
//
//     let withdrawn_jars = context.contract().withdraw_all(None).unwrap();
//
//     assert_eq!(withdrawn_jars.total_amount.0, 1000001);
//
//     assert_eq!(
//         withdrawn_jars
//             .jars
//             .iter()
//             .map(|j| j.withdrawn_amount.0)
//             .collect::<Vec<_>>(),
//         vec![mature_jar.principal]
//     );
//
//     let all_jars = context.contract().get_jars_for_account(alice);
//
//     assert_eq!(
//         all_jars.iter().map(|j| j.principal.0).collect::<Vec<_>>(),
//         vec![locked_jar.principal, immature_jar.principal]
//     );
//
//     assert_eq!(
//         all_jars.iter().map(|j| j.id.0).collect::<Vec<_>>(),
//         vec![locked_jar.id, immature_jar.id,]
//     );
// }
//
// #[test]
// fn batch_withdraw_all() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().enabled(true).lockup_term(MS_IN_YEAR);
//
//     let jars: Vec<_> = (0..8)
//         .map(|id| Jar::new(id).principal(PRINCIPAL + id as u128))
//         .collect();
//
//     let mut context = Context::new(admin).with_products(&[product]).with_jars(&jars);
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//
//     context.contract().claim_total(None);
//
//     let withdrawn_jars = context
//         .contract()
//         .withdraw_all(Some(vec![1.into(), 3.into(), 5.into()]))
//         .unwrap();
//
//     assert_eq!(withdrawn_jars.total_amount.0, 3000009);
//
//     let jars: Vec<_> = context
//         .contract()
//         .get_jars_for_account(alice)
//         .into_iter()
//         .map(|j| j.id.0)
//         .collect();
//
//     assert_eq!(jars, [0, 7, 2, 6, 4,]);
// }
