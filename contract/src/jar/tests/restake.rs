// use near_sdk::test_utils::test_env::{alice, bob, carol};
// use sweat_jar_model::{
//     api::{JarApi, ProductApi},
//     U32,
// };
//
// use crate::{
//     common::tests::Context,
//     jar::model::Jar,
//     product::model::Product,
//     test_utils::{admin, expect_panic},
// };
//
// #[test]
// fn restake_by_not_owner() {
//     let product = Product::new().with_allows_restaking(true);
//     let alice_jar = Jar::new(0);
//     let mut ctx = Context::new(admin())
//         .with_products(&[product])
//         .with_jars(&[alice_jar.clone()]);
//
//     ctx.switch_account(bob());
//     expect_panic(&ctx, "Account 'bob.near' doesn't exist", || {
//         ctx.contract().restake(U32(alice_jar.id));
//     });
//
//     expect_panic(&ctx, "Jars for account bob.near don't exist", || {
//         ctx.contract().restake_all(None);
//     });
//
//     ctx.switch_account(carol());
//     expect_panic(&ctx, "Account 'carol.near' doesn't exist", || {
//         ctx.contract().restake(U32(alice_jar.id));
//     });
//
//     expect_panic(&ctx, "Jars for account carol.near don't exist", || {
//         ctx.contract().restake_all(None);
//     });
// }
//
// #[test]
// #[should_panic(expected = "The jar is not mature yet")]
// fn restake_before_maturity() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(true);
//     let jar = Jar::new(0);
//     let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);
//
//     context.switch_account(&alice);
//     assert!(context.contract().restake_all(None).is_empty());
//     context.contract().restake(U32(jar.id));
// }
//
// #[test]
// #[should_panic(expected = "The product doesn't support restaking")]
// fn restake_when_restaking_is_not_supported() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(false);
//
//     let jar = Jar::new(0);
//     let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);
//
//     context.switch_account(&alice);
//     assert!(context.contract().restake_all(None).is_empty());
//     context.contract().restake(U32(jar.id));
// }
//
// #[test]
// #[should_panic(expected = "The product is disabled")]
// fn restake_with_disabled_product() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(true);
//     let jar = Jar::new(0);
//     let mut context = Context::new(admin.clone())
//         .with_products(&[product.clone()])
//         .with_jars(&[jar.clone()]);
//
//     context.switch_account(&admin);
//     context.with_deposit_yocto(1, |context| context.contract().set_enabled(product.id, false));
//
//     context.contract().products_cache.borrow_mut().clear();
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//     assert!(context.contract().restake_all(None).is_empty());
//     context.contract().restake(U32(jar.id));
// }
//
// #[test]
// #[should_panic(expected = "The jar is empty, nothing to restake")]
// fn restake_empty_jar() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(true);
//     let jar = Jar::new(0).principal(0);
//     let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//     assert!(context.contract().restake_all(None).is_empty());
//     context.contract().restake(U32(jar.id));
// }
//
// #[test]
// fn restake_after_maturity_for_restakable_product() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(true);
//     let jar = Jar::new(0);
//     let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//     context.contract().restake(U32(jar.id));
//
//     let alice_jars = context.contract().get_jars_for_account(alice);
//
//     assert_eq!(2, alice_jars.len());
//     assert_eq!(0, alice_jars.iter().find(|item| item.id.0 == 0).unwrap().principal.0);
//     assert_eq!(
//         1_000_000,
//         alice_jars.iter().find(|item| item.id.0 == 1).unwrap().principal.0
//     );
// }
//
// #[test]
// #[should_panic(expected = "The product doesn't support restaking")]
// fn restake_after_maturity_for_not_restakable_product() {
//     let alice = alice();
//     let admin = admin();
//
//     let product = Product::new().with_allows_restaking(false);
//     let jar = Jar::new(0);
//     let mut context = Context::new(admin.clone())
//         .with_products(&[product.clone()])
//         .with_jars(&[jar.clone()]);
//
//     context.set_block_timestamp_in_days(366);
//
//     context.switch_account(&alice);
//     assert!(context.contract().restake_all(None).is_empty());
//     context.contract().restake(U32(jar.id));
// }
