use fake::Fake;
use near_sdk::test_utils::test_env::alice;
use sweat_jar_model::{api::JarApi, MS_IN_YEAR};

use crate::{
    common::tests::Context,
    jar::model::Jar,
    test_utils::{admin, generate_product, JAR_ID_RANGE, PRINCIPAL},
};

#[test]
fn restake_all() {
    let alice = alice();
    let admin = admin();

    let restakable_product = generate_product("restakable_product")
        .with_allows_restaking(true)
        .lockup_term(MS_IN_YEAR);

    let disabled_restakable_product = generate_product("disabled_restakable_product")
        .with_allows_restaking(true)
        .enabled(false)
        .lockup_term(MS_IN_YEAR);

    let non_restakable_product = generate_product("non_restakable_product")
        .with_allows_restaking(false)
        .lockup_term(MS_IN_YEAR);

    let long_term_restakable_product = generate_product("long_term_restakable_product")
        .with_allows_restaking(true)
        .lockup_term(MS_IN_YEAR * 2);

    let restakable_jar_1 = Jar::generate(JAR_ID_RANGE.fake(), &alice, &restakable_product.id).principal(PRINCIPAL);
    let restakable_jar_2 = Jar::generate(JAR_ID_RANGE.fake(), &alice, &restakable_product.id).principal(PRINCIPAL);

    let disabled_jar = Jar::generate(JAR_ID_RANGE.fake(), &alice, &disabled_restakable_product.id).principal(PRINCIPAL);

    let non_restakable_jar =
        Jar::generate(JAR_ID_RANGE.fake(), &alice, &non_restakable_product.id).principal(PRINCIPAL);

    let long_term_jar =
        Jar::generate(JAR_ID_RANGE.fake(), &alice, &long_term_restakable_product.id).principal(PRINCIPAL);

    let mut context = Context::new(admin)
        .with_products(&[
            restakable_product,
            disabled_restakable_product,
            non_restakable_product,
            long_term_restakable_product,
        ])
        .with_jars(&[
            restakable_jar_1.clone(),
            restakable_jar_2.clone(),
            disabled_jar.clone(),
            non_restakable_jar.clone(),
            long_term_jar.clone(),
        ]);

    context.set_block_timestamp_in_days(366);

    context.switch_account(&alice);

    let restaked_jars = context.contract().restake_all();

    assert_eq!(restaked_jars.len(), 2);
    assert_eq!(restaked_jars.iter().map(|j| j.id.0).collect::<Vec<_>>(), vec![1, 2]);

    let all_jars = context.contract().get_jars_for_account(alice);

    assert_eq!(
        all_jars.iter().map(|j| j.id.0).collect::<Vec<_>>(),
        [
            restakable_jar_1.id,
            restakable_jar_2.id,
            disabled_jar.id,
            non_restakable_jar.id,
            long_term_jar.id,
            1,
            2,
        ]
    )
}

#[test]
fn restake_all_after_maturity_for_restakable_product_one_jar() {
    let alice = alice();
    let admin = admin();

    let product = generate_product("restakable_product")
        .with_allows_restaking(true)
        .lockup_term(MS_IN_YEAR);
    let jar = Jar::generate(0, &alice, &product.id).principal(PRINCIPAL);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_days(366);

    context.switch_account(&alice);
    let restaked = context.contract().restake_all().into_iter().next().unwrap();

    assert_eq!(restaked.account_id, alice);
    assert_eq!(restaked.principal.0, PRINCIPAL);
    assert_eq!(restaked.claimed_balance.0, 0);

    let alice_jars = context.contract().get_jars_for_account(alice);

    assert_eq!(2, alice_jars.len());
    assert_eq!(0, alice_jars.iter().find(|item| item.id.0 == 0).unwrap().principal.0);
    assert_eq!(
        PRINCIPAL,
        alice_jars.iter().find(|item| item.id.0 == 1).unwrap().principal.0
    );
}
