#![cfg(test)]

use std::collections::HashMap;

use common::tests::Context;
use fake::Fake;
use near_sdk::{
    json_types::U128,
    serde_json::{from_str, to_string},
    test_utils::test_env::{alice, bob},
};
use sweat_jar_model::{
    api::{ClaimApi, JarApi, PenaltyApi, ProductApi, WithdrawApi},
    jar::{AggregatedTokenAmountView, JarView},
    product::ApyView,
    TokenAmount, UDecimal, U32,
};

use super::*;
use crate::{
    common::test_data::set_test_log_events,
    product::{helpers::MessageSigner, model::DowngradableApy, tests::get_register_product_command},
    test_utils::{admin, UnwrapPromise},
};

#[test]
fn add_product_to_list_by_admin() {
    let admin = admin();
    let mut context = Context::new(admin.clone());

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| {
        context.contract().register_product(get_register_product_command())
    });

    let products = context.contract().get_products();
    assert_eq!(products.len(), 1);
    assert_eq!(products.first().unwrap().id, "product".to_string());
}

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn add_product_to_list_by_not_admin() {
    let admin = admin();
    let mut context = Context::new(admin);

    context.with_deposit_yocto(1, |context| {
        context.contract().register_product(get_register_product_command())
    });
}

#[test]
fn get_principle_with_no_jars() {
    let alice = alice();
    let admin = admin();
    let context = Context::new(admin);

    let principal = context.contract().get_total_principal(alice);
    assert_eq!(principal.total.0, 0);
}

#[test]
fn get_principal_with_single_jar() {
    let alice = alice();
    let admin = admin();

    let product = Product::new();
    let reference_jar = Jar::new(0).principal(100);
    let context = Context::new(admin)
        .with_products(&[product])
        .with_jars(&[reference_jar]);

    let principal = context.contract().get_total_principal(alice).total.0;
    assert_eq!(principal, 100);
}

#[test]
fn get_principal_with_multiple_jars() {
    let alice = alice();
    let admin = admin();

    let product = Product::new();
    let jars = &[
        Jar::new(0).principal(100),
        Jar::new(1).principal(200),
        Jar::new(2).principal(400),
    ];

    let context = Context::new(admin).with_products(&[product]).with_jars(jars);

    let principal = context.contract().get_total_principal(alice).total.0;
    assert_eq!(principal, 700);
}

#[test]
fn get_total_interest_with_no_jars() {
    let alice = alice();
    let admin = admin();

    let context = Context::new(admin);

    let interest = context.contract().get_total_interest(alice);

    assert_eq!(interest.amount.total.0, 0);
    assert_eq!(interest.amount.detailed, HashMap::new());
}

#[test]
fn get_total_interest_with_single_jar_after_30_minutes() {
    let alice = alice();
    let admin = admin();

    let product = Product::new();

    let jar_id = 0;
    let jar = Jar::new(jar_id).principal(100_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract().account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_minutes(30);

    let interest = context.contract().get_total_interest(alice);

    assert_eq!(interest.amount.total.0, 684);
    assert_eq!(interest.amount.detailed, HashMap::from([(U32(0), U128(684))]))
}

#[test]
fn get_total_interest_with_single_jar_on_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::new();

    let jar_id = 0;
    let jar = Jar::new(jar_id).principal(100_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract().account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_days(365);

    let interest = context.contract().get_total_interest(alice);

    assert_eq!(
        interest.amount,
        AggregatedTokenAmountView {
            detailed: [(U32(0), U128(12_000_000))].into(),
            total: U128(12_000_000)
        }
    )
}

#[test]
fn get_total_interest_with_single_jar_after_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::new();

    let jar_id = 0;
    let jar = Jar::new(jar_id).principal(100_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract().account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_days(400);

    let interest = context.contract().get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 12_000_000);
}

#[test]
fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::new();

    let jar_id = 0;
    let jar = Jar::new(jar_id).principal(100_000_000);
    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract().account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_days(182);

    let mut interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 5_983_561);

    context.switch_account(&alice);
    context.contract().claim_total(None);

    context.set_block_timestamp_in_days(365);

    interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 6_016_439);
}

#[test]
#[should_panic(expected = "Penalty is not applicable for constant APY")]
fn penalty_is_not_applicable_for_constant_apy() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product::new()
        .apy(Apy::Constant(UDecimal::new(20, 2)))
        .public_key(signer.public_key());
    let reference_jar = Jar::new(0).principal(100_000_000);

    let mut context = Context::new(admin.clone())
        .with_products(&[product])
        .with_jars(&[reference_jar]);

    context.switch_account(&admin);
    context.contract().set_penalty(alice, U32(0), true);
}

#[test]
fn get_total_interest_for_premium_with_penalty_after_half_term() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product::new()
        .apy(Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(20, 2),
            fallback: UDecimal::new(10, 2),
        }))
        .public_key(signer.public_key());
    let reference_jar = Jar::new(0).principal(100_000_000);

    let mut context = Context::new(admin.clone())
        .with_products(&[product])
        .with_jars(&[reference_jar]);

    context.set_block_timestamp_in_ms(15_768_000_000);

    let mut interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 10_000_000);

    context.switch_account(&admin);
    context.contract().set_penalty(alice.clone(), U32(0), true);

    context.set_block_timestamp_in_ms(31_536_000_000);

    interest = context.contract().get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 15_000_000);
}

#[test]
fn get_total_interest_for_premium_with_multiple_penalties_applied() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product::new()
        .apy(Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(23, 2),
            fallback: UDecimal::new(10, 2),
        }))
        .lockup_term(3_600_000)
        .public_key(signer.public_key());
    let reference_jar = Jar::new(0).principal(100_000_000_000_000_000_000_000);

    let mut context = Context::new(admin.clone())
        .with_products(&[product])
        .with_jars(&[reference_jar]);

    let products = context.contract().get_products();
    assert!(matches!(products.first().unwrap().apy, ApyView::Downgradable(_)));

    context.switch_account(&admin);

    context.set_block_timestamp_in_ms(270_000);
    context.contract().set_penalty(alice.clone(), U32(0), true);

    context.set_block_timestamp_in_ms(390_000);
    context.contract().set_penalty(alice.clone(), U32(0), false);

    context.set_block_timestamp_in_ms(1_264_000);
    context.contract().set_penalty(alice.clone(), U32(0), true);

    context.set_block_timestamp_in_ms(3_700_000);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_613_140_537_798_072_042);
}

#[test]
fn apply_penalty_in_batch() {
    let admin = admin();
    let alice = alice();
    let bob = bob();

    let signer = MessageSigner::new();
    let product = Product::new()
        .apy(Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(20, 2),
            fallback: UDecimal::new(10, 2),
        }))
        .public_key(signer.public_key());

    let alice_jars = (0..100).map(|id| Jar::new(id).principal(100_000_000));
    let bob_jars = (0..50).map(|id| Jar::new(id + 200).account_id(&bob).principal(100_000_000));

    let mut context = Context::new(admin.clone())
        .with_products(&[product])
        .with_jars(&alice_jars.chain(bob_jars).collect::<Vec<_>>());

    context.set_block_timestamp_in_days(182);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 997_260_200);

    let interest = context.contract().get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 498_630_100);

    context.switch_account(&admin);

    let alice_jars = context
        .contract()
        .get_jars_for_account(alice.clone())
        .into_iter()
        .map(|j| j.id)
        .collect();
    let bob_jars = context
        .contract()
        .get_jars_for_account(bob.clone())
        .into_iter()
        .map(|j| j.id)
        .collect();

    context
        .contract()
        .batch_set_penalty(vec![(alice.clone(), alice_jars), (bob.clone(), bob_jars)], true);

    context.set_block_timestamp_in_days(365);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_498_630_000);

    let interest = context.contract().get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 749_315_000);

    let alice_jars = context.contract().get_jars_for_account(alice);
    let bob_jars = context.contract().get_jars_for_account(bob);

    assert!(alice_jars.into_iter().chain(bob_jars).all(|jar| jar.is_penalty_applied));
}

#[test]
fn get_interest_after_withdraw() {
    let alice = alice();
    let admin = admin();

    let product = Product::new();
    let jar = Jar::new(0).principal(100_000_000);

    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_days(400);

    context.switch_account(&alice);
    context.contract().withdraw(U32(jar.id), None);

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(12_000_000, interest.amount.total.0);
}

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn unlock_not_by_manager() {
    let alice = alice();
    let admin = admin();

    let reference_product = Product::new();

    let mut reference_jar = Jar::new(0).product_id(&reference_product.id).principal(100);
    reference_jar.is_pending_withdraw = true;
    let jars = &[reference_jar];

    let mut context = Context::new(admin).with_products(&[reference_product]).with_jars(jars);

    context.switch_account(&alice);
    context.contract().unlock_jars_for_account(alice);
}

#[test]
fn unlock_by_manager() {
    let alice = alice();
    let admin = admin();

    let reference_product = Product::new();

    let reference_jar_id = 0;
    let mut reference_jar = Jar::new(0).product_id(&reference_product.id).principal(100);
    reference_jar.is_pending_withdraw = true;
    let jars = &[reference_jar];

    let mut context = Context::new(admin.clone())
        .with_products(&[reference_product])
        .with_jars(jars);

    assert!(
        context
            .contract()
            .get_jar(alice.clone(), reference_jar_id.into())
            .is_pending_withdraw
    );

    context.switch_account(&admin);
    context.contract().unlock_jars_for_account(alice.clone());

    assert!(
        !context
            .contract()
            .get_jar(alice.clone(), reference_jar_id.into())
            .is_pending_withdraw
    );
}

#[test]
fn test_u32() {
    let n = U32(12345678);

    assert_eq!(n, from_str(&to_string(&n).unwrap()).unwrap());
    assert_eq!(U32::from(12345678_u32), n);
}

#[test]
fn claim_often_vs_claim_once() {
    fn test(mut product: Product, principal: TokenAmount, days: u64, n: usize) {
        set_test_log_events(false);

        let alice: AccountId = format!("alice_{principal}_{days}_{n}").try_into().unwrap();
        let bob: AccountId = format!("bob_{principal}_{days}_{n}").try_into().unwrap();
        let admin: AccountId = format!("admin_{principal}_{days}_{n}").try_into().unwrap();

        product.id = format!("product_{principal}_{days}_{n}");

        let alice_jar = Jar::new(0)
            .product_id(&product.id)
            .account_id(&alice)
            .principal(principal);

        let bob_jar = Jar::new(1)
            .product_id(&product.id)
            .account_id(&bob)
            .principal(principal);

        let mut context = Context::new(admin)
            .with_products(&[product])
            .with_jars(&[alice_jar.clone(), bob_jar.clone()]);

        let mut bobs_claimed = 0;

        context.switch_account(&bob);

        for day in 0..days {
            context.set_block_timestamp_in_days(day);
            let claimed = context.contract().claim_total(None).unwrap();
            bobs_claimed += claimed.get_total().0;
        }

        let alice_interest = context.contract().get_total_interest(alice.clone()).amount.total.0;

        assert_eq!(alice_interest, bobs_claimed);
    }

    let product = Product::new();

    test(product.clone(), 10_000_000_000_000_000_000_000_000_000, 365, 0);

    for n in 1..10 {
        test(
            product.clone(),
            (1..10_000_000_000_000_000_000_000_000_000).fake(),
            (1..365).fake(),
            n,
        );
    }
}
