#![cfg(test)]

use std::collections::HashMap;

use near_sdk::{
    json_types::{U128, U64},
    AccountId,
};
use rstest::rstest;
use sweat_jar_model::{
    api::{AccountApi, ClaimApi, PenaltyApi, ProductApi, WithdrawApi},
    data::{
        deposit::DepositTicket,
        jar::{AggregatedTokenAmountView, Jar},
        product::{Apy, Product},
    },
};

use crate::{
    common::testing::{
        accounts::{admin, alice},
        Context,
    },
    feature::{
        account::model::test_utils::jar,
        product::model::test_utils::{
            product_1_hour_apy_downgradable_23_10_percent_protected, product_1_year_12_percent,
            product_1_year_apy_downgradable_20_10_percent_protected, product_disabled, BaseApy, ProtectedProduct,
        },
    },
};

#[rstest]
fn get_total_interest_with_no_jars(admin: AccountId, alice: AccountId) {
    let context = Context::new(admin);

    let interest = context.contract().get_total_interest(alice);

    assert_eq!(interest.amount.total.0, 0);
    assert_eq!(interest.amount.detailed, HashMap::new());
}

#[rstest]
fn get_total_interest_with_single_jar_after_30_minutes(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_minutes(30);

    let interest = context.contract().get_total_interest(alice);

    assert_eq!(interest.amount.total.0, 684);
    assert_eq!(
        interest.amount.detailed,
        HashMap::from([(product.id.clone(), U128(684))])
    );
}

#[rstest]
fn get_total_interest_with_single_jar_on_maturity(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
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
    );
}

#[rstest]
fn get_total_interest_with_single_jar_after_maturity(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_days(400);

    let interest = context.contract().get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 12_000_000);
}

#[rstest]
fn get_total_interest_with_single_jar_after_claim_on_half_term_and_maturity(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
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

#[rstest]
fn get_total_interest_for_premium_with_penalty_after_half_term(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_apy_downgradable_20_10_percent_protected)]
    ProtectedProduct { product, signer: _ }: ProtectedProduct,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
    let mut context = Context::new(admin.clone())
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_ms(15_768_000_000);

    let mut interest = context.contract().get_total_interest(alice.clone()).amount.total.0;
    assert_eq!(interest, 10_000_000);

    context.switch_account_to_manager();
    context.contract().set_penalty(alice.clone(), true);

    context.set_block_timestamp_in_ms(31_536_000_000);

    interest = context.contract().get_total_interest(alice).amount.total.0;
    assert_eq!(interest, 15_000_000);
}

#[rstest]
fn get_total_interest_for_premium_with_multiple_penalties_applied(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_hour_apy_downgradable_23_10_percent_protected)]
    ProtectedProduct { product, signer: _ }: ProtectedProduct,
    #[with(vec![(0, 100_000_000_000_000_000_000_000)])] jar: Jar,
) {
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

#[rstest]
#[should_panic(expected = "It's not possible to create new jars for this product")]
fn create_jar_for_disabled_product(admin: AccountId, alice: AccountId, #[from(product_disabled)] product: Product) {
    let context = Context::new(admin).with_products(&[product.clone()]);

    let ticket = DepositTicket {
        product_id: product.id,
        valid_until: U64(0),
        timezone: None,
    };

    context.contract().deposit(alice, ticket, 1_000_000, &None);
}

#[rstest]
fn get_interest_after_withdraw(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 100_000_000)])] jar: Jar,
) {
    let mut context = Context::new(admin)
        .with_products(&[product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())]);

    context.set_block_timestamp_in_days(400);

    context.switch_account(&alice);
    context.contract().withdraw(product.id.clone());

    let interest = context.contract().get_total_interest(alice.clone());
    assert_eq!(12_000_000, interest.amount.total.0);
}

#[rstest]
#[should_panic(expected = "Can be performed only by admin")]
fn unlock_not_by_manager(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 300_000_000)])] jar: Jar,
) {
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

#[rstest]
fn unlock_by_manager(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] product: Product,
    #[with(vec![(0, 300_000_000)])] jar: Jar,
) {
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

    context.switch_account_to_manager();
    context.contract().unlock_jars_for_account(alice.clone());

    assert!(
        !context
            .contract()
            .get_account(&alice)
            .get_jar(&product.id)
            .is_pending_withdraw
    );
}

mod signature_tests {
    use near_sdk::json_types::Base64VecU8;
    use sweat_jar_model::{data::deposit::DepositMessage, TokenAmount};

    use super::*;
    use crate::feature::product::model::test_utils::{product_1_year_apy_7_percent_protected, protected_product};

    #[rstest]
    fn verify_ticket_with_valid_signature_and_date(
        admin: AccountId,
        #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer }: ProtectedProduct,
    ) {
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 14_000_000;
        let ticket = DepositTicket {
            product_id: product.id,
            valid_until: U64(123_000_000),
            timezone: None,
        };

        let signature = signer.sign(context.get_deposit_message(&admin, &ticket, amount, 0).as_str());

        context
            .contract()
            .verify(&admin, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[rstest]
    fn sequential_deposits_with_tickets_with_valid_nonce(
        admin: AccountId,
        alice: AccountId,
        #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer }: ProtectedProduct,
    ) {
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 14_000_000;
        let ticket = DepositTicket {
            product_id: product.id,
            valid_until: U64(123_000_000),
            timezone: None,
        };

        let signature = signer.sign(context.get_deposit_message(&alice, &ticket, amount, 0).as_str());
        context
            .contract()
            .deposit(alice.clone(), ticket.clone(), amount, &Base64VecU8(signature).into());

        let signature = signer.sign(context.get_deposit_message(&alice, &ticket, amount, 1).as_str());
        context
            .contract()
            .deposit(alice.clone(), ticket, amount, &Base64VecU8(signature).into());

        let jars = context.contract().get_jars_for_account(alice);
        assert_eq!(2, jars.get_total_deposits_number());
    }

    #[rstest]
    #[should_panic(expected = "Not matching signature")]
    fn sequential_deposits_with_tickets_with_invalid_nonce(
        admin: AccountId,
        alice: AccountId,
        #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer }: ProtectedProduct,
    ) {
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 14_000_000;
        let ticket = DepositTicket {
            product_id: product.id,
            valid_until: U64(123_000_000),
            timezone: None,
        };

        let signature = signer.sign(context.get_deposit_message(&alice, &ticket, amount, 0).as_str());
        context
            .contract()
            .deposit(alice.clone(), ticket.clone(), amount, &Base64VecU8(signature).into());

        let signature = signer.sign(context.get_deposit_message(&alice, &ticket, amount, 0).as_str());
        context
            .contract()
            .deposit(alice.clone(), ticket, amount, &Base64VecU8(signature).into());
    }

    #[rstest]
    #[should_panic(expected = "Signature must be 64 bytes")]
    fn verify_ticket_with_invalid_signature(
        admin: AccountId,
        alice: AccountId,
        #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer: _ }: ProtectedProduct,
    ) {
        let context = Context::new(admin).with_products(&[product.clone()]);

        let amount = 1_000_000;
        let ticket = DepositTicket {
            product_id: product.id,
            valid_until: U64(100_000_000),
            timezone: None,
        };

        let signature: Vec<u8> = vec![0, 1, 2];

        context
            .contract()
            .verify(&alice, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[rstest]
    #[should_panic(expected = "Not matching signature")]
    fn verify_ticket_with_not_matching_signature(
        admin: AccountId,
        #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer }: ProtectedProduct,
        #[from(product_1_year_apy_downgradable_20_10_percent_protected)] ProtectedProduct {
            product: another_product,
            signer: _,
        }: ProtectedProduct,
    ) {
        let context = Context::new(admin.clone()).with_products(&[product, another_product.clone()]);

        let amount = 15_000_000;
        let ticket_for_another_product = DepositTicket {
            product_id: another_product.id,
            valid_until: U64(100_000_000),
            timezone: None,
        };

        // signature made for wrong product
        let signature = signer.sign(
            context
                .get_deposit_message(&admin, &ticket_for_another_product, amount, 0)
                .as_str(),
        );

        context.contract().verify(
            &admin,
            amount,
            &ticket_for_another_product,
            &Some(Base64VecU8(signature)),
        );
    }

    #[rstest]
    #[should_panic(expected = "Ticket is outdated")]
    fn verify_ticket_with_invalid_date(
        admin: AccountId,
        alice: AccountId,
        #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer }: ProtectedProduct,
    ) {
        let mut context = Context::new(admin).with_products(&[product.clone()]);

        context.set_block_timestamp_in_days(365);

        let amount = 5_000_000;
        let ticket = DepositTicket {
            product_id: product.id,
            valid_until: U64(100_000_000),
            timezone: None,
        };

        let signature = signer.sign(context.get_deposit_message(&alice, &ticket, amount, 0).as_str());

        context
            .contract()
            .verify(&alice, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[rstest]
    #[should_panic(expected = "Product non_existing_product is not found")]
    fn verify_ticket_with_not_existing_product(
        admin: AccountId,
        #[from(protected_product)]
        #[with("non_existing_product".to_string())]
        ProtectedProduct {
            product: not_existing_product,
            signer,
        }: ProtectedProduct,
    ) {
        let mut context = Context::new(admin.clone());

        context.switch_account_to_manager();

        let amount = 500_000;
        let ticket = DepositTicket {
            product_id: not_existing_product.id,
            valid_until: U64(100_000_000),
            timezone: None,
        };

        let signature = signer.sign(context.get_deposit_message(&admin, &ticket, amount, 0).as_str());

        context
            .contract()
            .verify(&admin, amount, &ticket, &Some(Base64VecU8(signature)));
    }

    #[rstest]
    #[should_panic(expected = "Signature is required")]
    fn verify_ticket_without_signature_when_required(
        admin: AccountId,
        #[from(product_1_year_apy_7_percent_protected)] ProtectedProduct { product, signer: _ }: ProtectedProduct,
    ) {
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 3_000_000;
        let ticket = DepositTicket {
            product_id: product.id,
            valid_until: U64(100_000_000),
            timezone: None,
        };

        context.contract().verify(&admin, amount, &ticket, &None);
    }

    #[rstest]
    fn verify_ticket_without_signature_when_not_required(
        admin: AccountId,
        #[from(product_1_year_12_percent)] product: Product,
    ) {
        let context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let amount = 4_000_000_000;
        let ticket = DepositTicket {
            product_id: product.id,
            valid_until: U64(0),
            timezone: None,
        };

        context.contract().verify(&admin, amount, &ticket, &None);
    }

    impl Context {
        fn get_deposit_message(
            &self,
            receiver_id: &AccountId,
            ticket: &DepositTicket,
            amount: TokenAmount,
            nonce: u32,
        ) -> String {
            DepositMessage::new(
                &self.owner,
                receiver_id,
                &ticket.product_id,
                amount,
                ticket.valid_until.0,
                nonce,
            )
            .to_string()
        }
    }
}
