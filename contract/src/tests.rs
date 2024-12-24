#![cfg(test)]

use common::tests::Context;
use fake::Fake;
use near_sdk::{
    json_types::U128,
    test_utils::test_env::{alice, bob},
};
use sweat_jar_model::{
    api::{ClaimApi, JarApi, PenaltyApi, ProductApi, WithdrawApi},
    jar::AggregatedTokenAmountView,
    product::{Apy, DowngradableApy, FixedProductTerms, Terms},
    TokenAmount, UDecimal, MS_IN_YEAR,
};

use super::*;
use crate::{
    common::test_data::set_test_log_events,
    jar::model::Jar,
    product::{helpers::MessageSigner, tests::get_testing_product},
    test_utils::{admin, UnwrapPromise},
};

#[test]
fn add_product_to_list_by_admin() {
    let admin = admin();
    let mut context = Context::new(admin.clone());

    context.switch_account(&admin);
    context.with_deposit_yocto(1, |context| context.contract().register_product(get_testing_product()));

    let products = context.contract().get_products();
    assert_eq!(products.len(), 1);
    assert_eq!(products.first().unwrap().id, "product".to_string());
}

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn add_product_to_list_by_not_admin() {
    let admin = admin();
    let mut context = Context::new(admin);

    context.with_deposit_yocto(1, |context| context.contract().register_product(get_testing_product()));
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

    let product = Product::default().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR.into(),
        apy: Apy::Constant(UDecimal::new(12000, 5)),
    }));
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_minutes(30);

    let interest = context.contract().get_total_interest(alice);

    assert_eq!(interest.amount.total.0, 684);
    assert_eq!(
        interest.amount.detailed,
        HashMap::from([(product.id.clone(), U128(684))])
    )
}

#[test]
fn get_total_interest_with_single_jar_on_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::default().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR.into(),
        apy: Apy::Constant(UDecimal::new(12000, 5)),
    }));
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_days(365);

    let interest = context.contract().get_total_interest(alice);

    assert_eq!(
        interest.amount,
        AggregatedTokenAmountView {
            detailed: [(product.id.clone(), U128(12_000_000))].into(),
            total: U128(12_000_000)
        }
    )
}

#[test]
fn get_total_interest_with_single_jar_after_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::default().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR.into(),
        apy: Apy::Constant(UDecimal::new(12000, 5)),
    }));
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_days(400);

    let interest = context.contract().get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 12_000_000);
}

#[test]
fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity() {
    let alice = alice();
    let admin = admin();

    let product = Product::default().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR.into(),
        apy: Apy::Constant(UDecimal::new(12000, 5)),
    }));
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

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
fn get_total_interest_for_premium_with_penalty_after_half_term() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product::default()
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR.into(),
            apy: Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20000, 5),
                fallback: UDecimal::new(10000, 5),
            }),
        }))
        .with_public_key(Some(signer.public_key()));
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(15_768_000_000);

    let mut interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 10_000_000);

    context.switch_account(&admin);
    context.contract().set_penalty(alice.clone(), true);

    context.set_block_timestamp_in_ms(31_536_000_000);

    interest = context.contract().get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 15_000_000);
}

#[test]
fn get_total_interest_for_premium_with_multiple_penalties_applied() {
    let alice = alice();
    let admin = admin();

    let signer = MessageSigner::new();
    let product = Product::default()
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: 3_600_000.into(),
            apy: Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(23000, 5),
                fallback: UDecimal::new(10000, 5),
            }),
        }))
        .with_public_key(Some(signer.public_key()));
    let jar = Jar::new().with_deposit(0, 100_000_000_000_000_000_000_000);
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    let products = context.contract().get_products();
    assert!(matches!(products.first().unwrap().get_base_apy(), Apy::Downgradable(_)));

    context.switch_account(&admin);

    context.set_block_timestamp_in_ms(270_000);
    context.contract().set_penalty(alice.clone(), true);

    context.set_block_timestamp_in_ms(390_000);
    context.contract().set_penalty(alice.clone(), false);

    context.set_block_timestamp_in_ms(1_264_000);
    context.contract().set_penalty(alice.clone(), true);

    context.set_block_timestamp_in_ms(3_700_000);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_613_140_537_798_072_044);
}

#[test]
fn apply_penalty_in_batch() {
    let admin = admin();
    let alice = alice();
    let bob = bob();

    let signer = MessageSigner::new();
    let product = Product::default()
        .with_terms(Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR.into(),
            apy: Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20000, 5),
                fallback: UDecimal::new(10000, 5),
            }),
        }))
        .with_public_key(Some(signer.public_key()));

    let alice_jar = Jar::new().with_deposit(0, 10_000_000_000);
    let bob_jar = Jar::new().with_deposit(0, 5_000_000_000);

    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), alice_jar)])
        .with_jars(&bob, &[(product.id.clone(), bob_jar)]);

    context.set_block_timestamp_in_ms(MS_IN_YEAR / 2);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_000_000_000);

    let interest = context.contract().get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 500_000_000);

    context.switch_account(&admin);

    context
        .contract()
        .batch_set_penalty(vec![alice.clone(), bob.clone()], true);

    context.set_block_timestamp_in_days(365);

    let interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 1_500_000_000);

    let interest = context.contract().get_total_interest(bob.clone()).amount.total.0;
    assert_eq!(interest, 750_000_000);

    assert!(context.contract().is_penalty_applied(alice));
    assert!(context.contract().is_penalty_applied(bob));
}

#[test]
fn get_interest_after_withdraw() {
    let alice = alice();
    let admin = admin();

    let product = Product::default().with_terms(Terms::Fixed(FixedProductTerms {
        lockup_term: MS_IN_YEAR.into(),
        apy: Apy::Constant(UDecimal::new(12000, 5)),
    }));
    let jar = Jar::new().with_deposit(0, 100_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_days(400);

    context.switch_account(&alice);
    context.contract().withdraw(product.id.clone());

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(12_000_000, interest.amount.total.0);
}

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn unlock_not_by_manager() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let jar = Jar::new().with_deposit(0, 300_000_000);
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);
    context
        .contract()
        .get_account_mut(&alice)
        .get_jar_mut(&product.id)
        .is_pending_withdraw = true;

    context.switch_account(&alice);
    context.contract().unlock_jars_for_account(alice);
}

#[test]
fn unlock_by_manager() {
    let alice = alice();
    let admin = admin();

    let product = Product::default();
    let jar = Jar::new().with_deposit(0, 300_000_000);
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);
    context
        .contract()
        .get_account_mut(&alice)
        .get_jar_mut(&product.id)
        .is_pending_withdraw = true;

    assert!(
        context
            .contract()
            .get_account(&alice)
            .get_jar(&product.id)
            .is_pending_withdraw
    );

    context.switch_account(&admin);
    context.contract().unlock_jars_for_account(alice.clone());

    assert!(
        !context
            .contract()
            .get_account(&alice)
            .get_jar(&product.id)
            .is_pending_withdraw
    );
}
#[test]
fn claim_often_vs_claim_once() {
    fn test(mut product: Product, principal: TokenAmount, days: u64, n: usize) {
        set_test_log_events(false);

        let alice: AccountId = format!("alice_{principal}_{days}_{n}").try_into().unwrap();
        let bob: AccountId = format!("bob_{principal}_{days}_{n}").try_into().unwrap();
        let admin: AccountId = format!("admin_{principal}_{days}_{n}").try_into().unwrap();

        product.id = format!("product_{principal}_{days}_{n}");

        let alice_jar = Jar::new().with_deposit(0, principal);
        let bob_jar = Jar::new().with_deposit(0, principal);

        let mut context = Context::new(admin)
            .with_products(&[product.clone()])
            .with_jars(&alice, &[(product.id.clone(), alice_jar)])
            .with_jars(&bob, &[(product.id.clone(), bob_jar)]);

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

    let product = Product::default();

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
