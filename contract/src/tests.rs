#![cfg(test)]

use std::collections::HashMap;

use common::tests::Context;
use model::{jar::JarView, AggregatedTokenAmountView, U32};
use near_sdk::{json_types::U128, test_utils::accounts};

use super::*;
use crate::{
    claim::api::ClaimApi,
    common::{udecimal::UDecimal, MS_IN_YEAR},
    jar::api::JarApi,
    penalty::api::PenaltyApi,
    product::{api::*, helpers::MessageSigner, model::DowngradableApy, tests::get_register_product_command},
    withdraw::api::WithdrawApi,
};

#[test]
fn add_product_to_list_by_admin() {
    let admin = accounts(0);
    let mut context = Context::new(admin.clone());

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| {
        context.contract.register_product(get_register_product_command())
    });

    let products = context.contract.get_products();
    assert_eq!(products.len(), 1);
    assert_eq!(products.first().unwrap().id, "product".to_string());
}

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn add_product_to_list_by_not_admin() {
    let admin = accounts(0);
    let mut context = Context::new(admin);

    context.with_deposit_yocto(1, |context| {
        context.contract.register_product(get_register_product_command())
    });
}

#[test]
fn get_principle_with_no_jars() {
    let alice = accounts(0);
    let admin = accounts(1);
    let context = Context::new(admin);

    let principal = context.contract.get_total_principal(alice);
    assert_eq!(principal.total.0, 0);
}

#[test]
fn get_principal_with_single_jar() {
    let alice = accounts(0);
    let admin = accounts(1);

    let reference_product = generate_product();
    let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(100);
    let context = Context::new(admin)
        .with_products(&[reference_product])
        .with_jars(&[reference_jar]);

    let principal = context.contract.get_total_principal(alice).total.0;
    assert_eq!(principal, 100);
}

#[test]
fn get_principal_with_multiple_jars() {
    let alice = accounts(0);
    let admin = accounts(1);

    let reference_product = generate_product();
    let jars = &[
        Jar::generate(0, &alice, &reference_product.id).principal(100),
        Jar::generate(1, &alice, &reference_product.id).principal(200),
        Jar::generate(2, &alice, &reference_product.id).principal(400),
    ];

    let context = Context::new(admin).with_products(&[reference_product]).with_jars(jars);

    let principal = context.contract.get_total_principal(alice).total.0;
    assert_eq!(principal, 700);
}

#[test]
fn get_total_interest_with_no_jars() {
    let alice = accounts(0);
    let admin = accounts(1);

    let context = Context::new(admin);

    let interest = context.contract.get_total_interest(alice);

    assert_eq!(interest.amount.total.0, 0);
    assert_eq!(interest.amount.detailed, HashMap::new());
}

#[test]
fn get_total_interest_with_single_jar_after_30_minutes() {
    let alice = accounts(0);
    let admin = accounts(1);

    let reference_product = generate_product();

    let jar_id = 0;
    let jar = Jar::generate(jar_id, &alice, &reference_product.id).principal(100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[reference_product])
        .with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract.account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_minutes(30);

    let interest = context.contract.get_total_interest(alice);

    assert_eq!(interest.amount.total.0, 684);
    assert_eq!(interest.amount.detailed, HashMap::from([(U32(0), U128(684))]))
}

#[test]
fn get_total_interest_with_single_jar_on_maturity() {
    let alice = accounts(0);
    let admin = accounts(1);

    let reference_product = generate_product();

    let jar_id = 0;
    let jar = Jar::generate(jar_id, &alice, &reference_product.id).principal(100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[reference_product])
        .with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract.account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_days(365);

    let interest = context.contract.get_total_interest(alice);

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
    let alice = accounts(0);
    let admin = accounts(1);

    let reference_product = generate_product();

    let jar_id = 0;
    let jar = Jar::generate(jar_id, &alice, &reference_product.id).principal(100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[reference_product])
        .with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract.account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_days(400);

    let interest = context.contract.get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 12_000_000);
}

#[test]
fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity() {
    let alice = accounts(0);
    let admin = accounts(1);

    let reference_product = generate_product();

    let jar_id = 0;
    let jar = Jar::generate(jar_id, &alice, &reference_product.id).principal(100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[reference_product])
        .with_jars(&[jar.clone()]);

    let contract_jar = JarView::from(context.contract.account_jars.get(&alice).unwrap().get_jar(jar_id));
    assert_eq!(JarView::from(jar), contract_jar);

    context.set_block_timestamp_in_days(182);

    let mut interest = context.contract.get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 5_983_561);

    context.switch_account(&alice);
    context.contract.claim_total(None);

    context.set_block_timestamp_in_days(365);

    interest = context.contract.get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 6_016_438);
}

#[test]
#[should_panic(expected = "Penalty is not applicable for constant APY")]
fn penalty_is_not_applicable_for_constant_apy() {
    let alice = accounts(0);
    let admin = accounts(1);

    let signer = MessageSigner::new();
    let reference_product = Product::generate("premium_product")
        .enabled(true)
        .apy(Apy::Constant(UDecimal::new(20, 2)))
        .public_key(signer.public_key());
    let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(100_000_000);

    let mut context = Context::new(admin.clone())
        .with_products(&[reference_product])
        .with_jars(&[reference_jar]);

    context.switch_account(&admin);
    context.contract.set_penalty(alice, U32(0), true);
}

#[test]
fn get_total_interest_for_premium_with_penalty_after_half_term() {
    let alice = accounts(0);
    let admin = accounts(1);

    let signer = MessageSigner::new();
    let reference_product = Product::generate("premium_product")
        .enabled(true)
        .apy(Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(20, 2),
            fallback: UDecimal::new(10, 2),
        }))
        .public_key(signer.public_key());
    let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(100_000_000);

    let mut context = Context::new(admin.clone())
        .with_products(&[reference_product])
        .with_jars(&[reference_jar]);

    context.set_block_timestamp_in_ms(15_768_000_000);

    let mut interest = context.contract.get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 10_000_000);

    context.switch_account(&admin);
    context.contract.set_penalty(alice.clone(), U32(0), true);

    context.set_block_timestamp_in_ms(31_536_000_000);

    interest = context.contract.get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 15_000_000);
}

#[test]
fn get_total_interest_for_premium_with_multiple_penalties_applied() {
    let alice = accounts(0);
    let admin = accounts(1);

    let signer = MessageSigner::new();
    let reference_product = Product::generate("lux_product")
        .enabled(true)
        .apy(Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(23, 2),
            fallback: UDecimal::new(10, 2),
        }))
        .lockup_term(3_600_000)
        .public_key(signer.public_key());
    let reference_jar = Jar::generate(0, &alice, &reference_product.id).principal(100_000_000_000_000_000_000_000);

    let mut context = Context::new(admin.clone())
        .with_products(&[reference_product])
        .with_jars(&[reference_jar]);

    context.switch_account(&admin);

    context.set_block_timestamp_in_ms(270_000);
    context.contract.set_penalty(alice.clone(), U32(0), true);

    context.set_block_timestamp_in_ms(390_000);
    context.contract.set_penalty(alice.clone(), U32(0), false);

    context.set_block_timestamp_in_ms(1_264_000);
    context.contract.set_penalty(alice.clone(), U32(0), true);

    context.set_block_timestamp_in_ms(3_700_000);

    let interest = context.contract.get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_613_140_537_798_072_042);
}

#[test]
fn apply_penalty_in_batch() {
    let admin = accounts(0);
    let alice = accounts(1);
    let bob = accounts(2);

    let product_id = "premium_product";

    let signer = MessageSigner::new();
    let reference_product = Product::generate(product_id)
        .enabled(true)
        .apy(Apy::Downgradable(DowngradableApy {
            default: UDecimal::new(20, 2),
            fallback: UDecimal::new(10, 2),
        }))
        .public_key(signer.public_key());

    let alice_jars = (0..100).map(|id| Jar::generate(id, &alice, product_id).principal(100_000_000));
    let bob_jars = (0..50).map(|id| Jar::generate(id + 200, &bob, product_id).principal(100_000_000));

    let mut context = Context::new(admin.clone())
        .with_products(&[reference_product])
        .with_jars(&alice_jars.chain(bob_jars).collect::<Vec<_>>());

    context.set_block_timestamp_in_days(182);

    let interest = context.contract.get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 997_260_200);

    let interest = context.contract.get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 498_630_100);

    context.switch_account(&admin);

    let alice_jars = context
        .contract
        .get_jars_for_account(alice.clone())
        .into_iter()
        .map(|j| j.id)
        .collect();
    let bob_jars = context
        .contract
        .get_jars_for_account(bob.clone())
        .into_iter()
        .map(|j| j.id)
        .collect();

    context
        .contract
        .batch_set_penalty(vec![(alice.clone(), alice_jars), (bob.clone(), bob_jars)], true);

    context.set_block_timestamp_in_days(365);

    let interest = context.contract.get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_498_630_000);

    let interest = context.contract.get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 749_315_000);

    assert!(context
        .contract
        .get_jars_for_account(alice)
        .into_iter()
        .chain(context.contract.get_jars_for_account(bob).into_iter())
        .all(|jar| jar.is_penalty_applied == true))
}

#[test]
fn get_interest_after_withdraw() {
    let alice = accounts(0);
    let admin = accounts(1);

    let product = generate_product();
    let jar = Jar::generate(0, &alice, &product.id).principal(100_000_000);

    let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar.clone()]);

    context.set_block_timestamp_in_days(400);

    context.switch_account(&alice);
    context.contract.withdraw(U32(jar.id), None);

    let interest = context.contract.get_total_interest(alice.clone());
    assert_eq!(12_000_000, interest.amount.total.0);
}

fn generate_product() -> Product {
    Product::generate("product")
        .enabled(true)
        .lockup_term(MS_IN_YEAR)
        .apy(Apy::Constant(UDecimal::new(12, 2)))
}
