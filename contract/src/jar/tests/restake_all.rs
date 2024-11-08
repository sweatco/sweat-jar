// TODO: fix test
// use near_sdk::{
//     json_types::{U128, U64},
//     test_utils::test_env::alice,
// };
// use sweat_jar_model::{
//     api::{ClaimApi, JarApi, ProductApi},
//     jar::JarView,
//     MS_IN_DAY, MS_IN_MINUTE, MS_IN_YEAR,
// };
//
// use crate::{
//     common::tests::Context,
//     jar::{model::JarV2, view::create_synthetic_jar_id},
//     product::model::{
//         v2::{Apy, FixedProductTerms, Terms},
//         ProductV2,
//     },
//     test_utils::{admin, PRINCIPAL},
// };
//
// #[test]
// fn restake_all() {
//     let alice = alice();
//     let admin = admin();
//
//     let regular_product = ProductV2::new().id("regular_product");
//     let regular_product_to_disable = ProductV2::new().id("disabled_product");
//     let long_term_product = ProductV2::new()
//         .id("long_term_product")
//         .with_terms(Terms::Fixed(FixedProductTerms {
//             lockup_term: MS_IN_YEAR * 2,
//             apy: Apy::new_downgradable(),
//         }));
//     let long_term_product_to_disable = ProductV2::new()
//         .id("long_term_disabled_product")
//         .with_terms(Terms::Fixed(FixedProductTerms {
//             lockup_term: MS_IN_YEAR * 2,
//             apy: Apy::new_downgradable(),
//         }));
//
//     let regular_product_jar = JarV2::new()
//         .with_deposit(0, PRINCIPAL)
//         .with_deposit(MS_IN_DAY, PRINCIPAL);
//     let product_to_disable_jar = JarV2::new().with_deposit(0, PRINCIPAL);
//     let long_term_product_jar = JarV2::new().with_deposit(0, PRINCIPAL);
//     let long_term_product_to_disable_jar = JarV2::new().with_deposit(0, PRINCIPAL);
//
//     let mut context = Context::new(admin.clone())
//         .with_products(&[
//             regular_product.clone(),
//             regular_product_to_disable.clone(),
//             long_term_product.clone(),
//             long_term_product_to_disable.clone(),
//         ])
//         .with_jars(
//             &alice,
//             &[
//                 (regular_product.id.clone(), regular_product_jar),
//                 (regular_product_to_disable.id.clone(), product_to_disable_jar),
//                 (long_term_product.id.clone(), long_term_product_jar),
//                 (
//                     long_term_product_to_disable.id.clone(),
//                     long_term_product_to_disable_jar,
//                 ),
//             ],
//         );
//
//     context.set_block_timestamp_in_ms(MS_IN_YEAR);
//
//     context.switch_account(&admin);
//     context.with_deposit_yocto(1, |context| {
//         context
//             .contract()
//             .set_enabled(regular_product_to_disable.id.clone(), false)
//     });
//     context.with_deposit_yocto(1, |context| {
//         context
//             .contract()
//             .set_enabled(long_term_product_to_disable.id.clone(), false)
//     });
//
//     let restaking_time = MS_IN_YEAR + 2 * MS_IN_DAY;
//     context.set_block_timestamp_in_ms(restaking_time);
//
//     context.switch_account(&alice);
//     let restaked_jars = context.contract().restake_all(None);
//     assert_eq!(restaked_jars.len(), 1);
//     assert_eq!(
//         restaked_jars.first().unwrap(),
//         &(regular_product.id.clone(), PRINCIPAL * 2)
//     );
//
//     let all_jars = context.contract().get_jars_for_account(alice);
//     let all_jar_ids = all_jars.iter().map(|j| j.id.clone()).collect::<Vec<_>>();
//     assert!(all_jar_ids.contains(&create_synthetic_jar_id(regular_product.id, restaking_time)));
//     assert!(all_jar_ids.contains(&create_synthetic_jar_id(regular_product_to_disable.id, 0)));
//     assert!(all_jar_ids.contains(&create_synthetic_jar_id(long_term_product.id, 0)));
//     assert!(all_jar_ids.contains(&create_synthetic_jar_id(long_term_product_to_disable.id, 0)));
// }
//
// #[test]
// fn restake_all_after_maturity_for_restakable_product_one_jar() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = ProductV2::new();
//     let jar = JarV2::new().with_deposit(0, PRINCIPAL);
//     let mut context = Context::new(admin)
//         .with_products(&[product.clone()])
//         .with_jars(&alice, &[(product.id.clone(), jar.clone())]);
//
//     let restake_time = MS_IN_YEAR + MS_IN_MINUTE;
//     context.set_block_timestamp_in_ms(restake_time);
//
//     context.switch_account(&alice);
//     let result = context.contract().restake_all(None);
//
//     assert_eq!(1, result.len());
//     assert_eq!((product.id.clone(), PRINCIPAL), result.first().unwrap().clone());
//
//     let alice_jars = context.contract().get_jars_for_account(alice);
//     assert_eq!(1, alice_jars.len());
//     assert_eq!(PRINCIPAL, alice_jars.first().unwrap().principal.0);
//     assert_eq!(restake_time, alice_jars.first().unwrap().created_at.0);
// }
//
// #[test]
// fn batch_restake_all() {
//     let alice = alice();
//     let admin = admin();
//
//     let product_one = ProductV2::new().id("product_one");
//     let product_two = ProductV2::new().id("product_two");
//     let product_three = ProductV2::new().id("product_three");
//
//     let product_one_jar_deposit_first = (0, PRINCIPAL);
//     let product_one_jar_deposit_second = (MS_IN_DAY, 5 * PRINCIPAL);
//     let product_two_jar_deposit = (0, 2 * PRINCIPAL);
//     let product_three_jar_deposit = (0, 3 * PRINCIPAL);
//
//     let product_one_jar = JarV2::new()
//         .with_deposit(product_one_jar_deposit_first.0, product_one_jar_deposit_first.1)
//         .with_deposit(product_one_jar_deposit_second.0, product_one_jar_deposit_second.1);
//     let product_two_jar = JarV2::new().with_deposit(product_two_jar_deposit.0, product_two_jar_deposit.1);
//     let product_three_jar = JarV2::new().with_deposit(product_three_jar_deposit.0, product_three_jar_deposit.1);
//
//     let mut context = Context::new(admin)
//         .with_products(&[product_one.clone(), product_two.clone(), product_three.clone()])
//         .with_jars(
//             &alice,
//             &[
//                 (product_one.id.clone(), product_one_jar),
//                 (product_two.id.clone(), product_two_jar),
//                 (product_three.id.clone(), product_three_jar),
//             ],
//         );
//
//     let restake_time = MS_IN_YEAR + MS_IN_DAY;
//     context.set_block_timestamp_in_ms(restake_time);
//
//     context.switch_account(&alice);
//
//     context.contract().claim_total(None);
//
//     let result: Vec<_> = context
//         .contract()
//         .restake_all(Some(vec![product_one.id.clone(), product_two.id.clone()]));
//
//     assert_eq!(2, result.len());
//     assert!(result.contains(&(product_one.id.clone(), product_one_jar_deposit_first.1)));
//     assert!(result.contains(&(product_two.id.clone(), product_two_jar_deposit.1)));
//
//     let mut jars: Vec<_> = context.contract().get_jars_for_account(alice);
//     jars.sort_by(|a, b| b.id.cmp(&a.id));
//     assert_eq!(4, jars.len());
//
//     let mut expected_jars = vec![
//         JarView {
//             id: create_synthetic_jar_id(product_one.id.clone(), restake_time),
//             product_id: product_one.id.clone(),
//             created_at: U64(restake_time),
//             principal: U128(product_one_jar_deposit_first.1),
//         },
//         JarView {
//             id: create_synthetic_jar_id(product_one.id.clone(), product_one_jar_deposit_second.0),
//             product_id: product_one.id.clone(),
//             created_at: U64(product_one_jar_deposit_second.0),
//             principal: U128(product_one_jar_deposit_second.1),
//         },
//         JarView {
//             id: create_synthetic_jar_id(product_two.id.clone(), restake_time),
//             product_id: product_two.id.clone(),
//             created_at: U64(restake_time),
//             principal: U128(product_two_jar_deposit.1),
//         },
//         JarView {
//             id: create_synthetic_jar_id(product_three.id.clone(), 0),
//             product_id: product_three.id.clone(),
//             created_at: U64(0),
//             principal: U128(product_three_jar_deposit.1),
//         },
//     ];
//     expected_jars.sort_by(|a, b| b.id.cmp(&a.id));
//     assert_eq!(jars, expected_jars);
// }
