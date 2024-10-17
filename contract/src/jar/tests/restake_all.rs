use near_sdk::test_utils::test_env::alice;
use sweat_jar_model::{
    api::{JarApi, ProductApi},
    MS_IN_DAY, MS_IN_YEAR,
};

use crate::{
    common::tests::Context,
    jar::{model::JarV2, view::create_synthetic_jar_id},
    product::model::{
        v2::{Apy, FixedProductTerms, Terms},
        ProductV2,
    },
    test_utils::{admin, PRINCIPAL},
};

#[test]
fn restake_all() {
    let alice = alice();
    let admin = admin();

    let regular_product = ProductV2::new().id("regular_product");
    let regular_product_to_disable = ProductV2::new().id("disabled_product");
    let long_term_product = ProductV2::new()
        .id("long_term_product")
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR * 2,
            apy: Apy::new_downgradable(),
        }));
    let long_term_product_to_disable = ProductV2::new()
        .id("long_term_disabled_product")
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR * 2,
            apy: Apy::new_downgradable(),
        }));

    let regular_product_jar = JarV2::new()
        .with_deposit(0, PRINCIPAL)
        .with_deposit(MS_IN_DAY, PRINCIPAL);
    let product_to_disable_jar = JarV2::new().with_deposit(0, PRINCIPAL);
    let long_term_product_jar = JarV2::new().with_deposit(0, PRINCIPAL);
    let long_term_product_to_disable_jar = JarV2::new().with_deposit(0, PRINCIPAL);

    let mut context = Context::new(admin.clone())
        .with_products(&[
            regular_product.clone(),
            regular_product_to_disable.clone(),
            long_term_product.clone(),
            long_term_product_to_disable.clone(),
        ])
        .with_jars(
            &alice,
            &[
                (regular_product.id.clone(), regular_product_jar),
                (regular_product_to_disable.id.clone(), product_to_disable_jar),
                (long_term_product.id.clone(), long_term_product_jar),
                (
                    long_term_product_to_disable.id.clone(),
                    long_term_product_to_disable_jar,
                ),
            ],
        );

    context.set_block_timestamp_in_ms(MS_IN_YEAR);

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| {
        context
            .contract()
            .set_enabled(regular_product_to_disable.id.clone(), false)
    });
    context.with_deposit_yocto(1, |context| {
        context
            .contract()
            .set_enabled(long_term_product_to_disable.id.clone(), false)
    });

    let restaking_time = MS_IN_YEAR + 2 * MS_IN_DAY;
    context.set_block_timestamp_in_ms(restaking_time);

    context.switch_account(&alice);
    let restaked_jars = context.contract().restake_all(None);
    assert_eq!(restaked_jars.len(), 1);
    assert_eq!(
        restaked_jars.first().unwrap(),
        &(regular_product.id.clone(), PRINCIPAL * 2)
    );

    let all_jars = context.contract().get_jars_for_account(alice);
    let all_jar_ids = all_jars.iter().map(|j| j.id.clone()).collect::<Vec<_>>();
    assert!(all_jar_ids.contains(&create_synthetic_jar_id(regular_product.id, restaking_time)));
    assert!(all_jar_ids.contains(&create_synthetic_jar_id(regular_product_to_disable.id, 0)));
    assert!(all_jar_ids.contains(&create_synthetic_jar_id(long_term_product.id, 0)));
    assert!(all_jar_ids.contains(&create_synthetic_jar_id(long_term_product_to_disable.id, 0)));
}

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
