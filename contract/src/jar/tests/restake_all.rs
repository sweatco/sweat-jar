// use near_sdk::test_utils::test_env::alice;
// use sweat_jar_model::{
//     api::{ClaimApi, JarApi},
//     MS_IN_YEAR,
// };
//
// use crate::{
//     common::tests::Context,
//     jar::model::Jar,
//     product::model::Product,
//     test_utils::{admin, PRINCIPAL},
// };
//
// #[test]
// fn restake_all() {
//     let alice = alice();
//     let admin = admin();
//
//     let restakable_product = Product::new().id("restakable_product").with_allows_restaking(true);
//
//     let disabled_restakable_product = Product::new()
//         .id("disabled_restakable_product")
//         .with_allows_restaking(true)
//         .enabled(false);
//
//     let non_restakable_product = Product::new().id("non_restakable_product").with_allows_restaking(false);
//
//     let long_term_restakable_product = Product::new()
//         .id("long_term_restakable_product")
//         .with_allows_restaking(true)
//         .lockup_term(MS_IN_YEAR * 2);
//
//     let restakable_jar_1 = Jar::new(0).product_id(&restakable_product.id).principal(PRINCIPAL);
//     let restakable_jar_2 = Jar::new(1).product_id(&restakable_product.id).principal(PRINCIPAL);
//
//     let disabled_jar = Jar::new(2)
//         .product_id(&disabled_restakable_product.id)
//         .principal(PRINCIPAL);
//
//     let non_restakable_jar = Jar::new(3).product_id(&non_restakable_product.id).principal(PRINCIPAL);
//
//     let long_term_jar = Jar::new(4)
//         .product_id(&long_term_restakable_product.id)
//         .principal(PRINCIPAL);
//
//     let mut context = Context::new(admin)
//         .with_products(&[
//             restakable_product,
//             disabled_restakable_product,
//             non_restakable_product,
//             long_term_restakable_product,
//         ])
//         .with_jars(&[
//             restakable_jar_1.clone(),
//             restakable_jar_2.clone(),
//             disabled_jar.clone(),
//             non_restakable_jar.clone(),
//             long_term_jar.clone(),
//         ]);
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//
//     let restaked_jars = context.contract().restake_all(None);
//
//     assert_eq!(restaked_jars.len(), 2);
//     assert_eq!(
//         restaked_jars.iter().map(|j| j.id.0).collect::<Vec<_>>(),
//         // 4 was last jar is, so 2 new restaked jars will have ids 5 and 6
//         vec![5, 6]
//     );
//
//     let all_jars = context.contract().get_jars_for_account(alice);
//
//     assert_eq!(
//         all_jars.iter().map(|j| j.id.0).collect::<Vec<_>>(),
//         [
//             restakable_jar_1.id,
//             restakable_jar_2.id,
//             disabled_jar.id,
//             non_restakable_jar.id,
//             long_term_jar.id,
//             5,
//             6,
//         ]
//     )
// }
//
// #[test]
// fn restake_all_after_maturity_for_restakable_product_one_jar() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(true);
//     let jar = Jar::new(0).principal(PRINCIPAL);
//     let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//     let restaked = context.contract().restake_all(None).into_iter().next().unwrap();
//
//     assert_eq!(restaked.account_id, alice);
//     assert_eq!(restaked.principal.0, PRINCIPAL);
//     assert_eq!(restaked.claimed_balance.0, 0);
//
//     let alice_jars = context.contract().get_jars_for_account(alice);
//
//     assert_eq!(2, alice_jars.len());
//     assert_eq!(0, alice_jars.iter().find(|item| item.id.0 == 0).unwrap().principal.0);
//     assert_eq!(
//         PRINCIPAL,
//         alice_jars.iter().find(|item| item.id.0 == 1).unwrap().principal.0
//     );
// }
//
// #[test]
// fn batch_restake_all() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(true).lockup_term(MS_IN_YEAR);
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
//     let restaked: Vec<_> = context
//         .contract()
//         .restake_all(Some(vec![1.into(), 2.into(), 5.into()]))
//         .into_iter()
//         .map(|j| j.id.0)
//         .collect();
//
//     assert_eq!(restaked, [8, 9, 10]);
//
//     let jars: Vec<_> = context
//         .contract()
//         .get_jars_for_account(alice)
//         .into_iter()
//         .map(|j| j.id.0)
//         .collect();
//
//     assert_eq!(jars, [0, 7, 8, 3, 4, 9, 6, 10,]);
// }
