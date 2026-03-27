use near_sdk::{env, json_types::Base64VecU8, AccountId};
use sweat_jar_model::{
    data::{
        deposit::DepositTicket,
        product::ProductAssertions,
    },
    TokenAmount,
};

use crate::{
    common::event::{emit, EventKind},
    Contract,
};

impl Contract {
    pub(crate) fn airdrop(
        &mut self,
        ticket: DepositTicket,
        amount_per_receiver: TokenAmount,
        receivers: Vec<AccountId>,
        signature: Option<&Base64VecU8>,
    ) {
        let product = self.get_product(&ticket.product_id);

        product.assert_enabled();
        product.assert_cap(amount_per_receiver);
        self.verify_airdrop(&ticket, amount_per_receiver, &receivers, signature);

        let now = env::block_timestamp_ms();
        for account_id in receivers {
            let account = self.get_or_create_account_mut(&account_id);
            // Nonce is intentionally NOT incremented — airdrop is a manager action
            if product.terms.is_score_based() {
                account.try_set_timezone(ticket.timezone);
            }
            account.deposit(&ticket.product_id, amount_per_receiver, None);
            account.update_jar_cache(&product, now);
            emit(EventKind::Deposit(
                account_id,
                (ticket.product_id.clone(), amount_per_receiver.into()),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::{json_types::U128, serde_json::json, AccountId};
    use rstest::rstest;
    use sweat_jar_model::{
        data::{
            deposit::AirdropMessage,
            product::Product,
        },
        signer::test_utils::Base64String,
        Timezone,
    };

    use crate::{
        common::testing::{
            accounts::{admin, alice, bob},
            Context,
        },
        feature::product::model::test_utils::{
            product, product_1_year_12_cap_score_based, protected_product, ProtectedProduct,
        },
    };

    fn airdrop_msg(product_id: &str, receivers: &[AccountId]) -> String {
        airdrop_msg_with_timezone(product_id, receivers, None)
    }

    fn airdrop_msg_with_timezone(product_id: &str, receivers: &[AccountId], timezone: Option<Timezone>) -> String {
        json!({
            "type": "airdrop",
            "data": {
                "ticket": {
                    "product_id": product_id,
                    "valid_until": "0",
                    "timezone": timezone,
                },
                "receivers": receivers,
            }
        })
        .to_string()
    }

    #[rstest]
    fn airdrop_basic(admin: AccountId, alice: AccountId, bob: AccountId, product: Product) {
        let amount_per_receiver = 1_000_000u128;
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        context.switch_account_to_ft_contract_account();
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(amount_per_receiver * 2),
            airdrop_msg(&product.id, &[alice.clone(), bob.clone()]),
        );

        let contract = context.contract();

        let alice_principal = contract.get_account(&alice).get_jar(&product.id).total_principal();
        assert_eq!(amount_per_receiver, alice_principal);

        let bob_principal = contract.get_account(&bob).get_jar(&product.id).total_principal();
        assert_eq!(amount_per_receiver, bob_principal);
    }

    #[rstest]
    fn airdrop_nonce_not_incremented(admin: AccountId, alice: AccountId, product: Product) {
        let amount_per_receiver = 1_000_000u128;
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        context.switch_account_to_ft_contract_account();
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(amount_per_receiver),
            airdrop_msg(&product.id, &[alice.clone()]),
        );

        let nonce = context.contract().get_account(&alice).nonce;
        assert_eq!(0, nonce, "Nonce must not be incremented by airdrop");
    }

    #[rstest]
    fn airdrop_score_based_sets_timezone(
        admin: AccountId,
        alice: AccountId,
        bob: AccountId,
        product_1_year_12_cap_score_based: Product,
    ) {
        let product = product_1_year_12_cap_score_based;
        let amount_per_receiver = 1_000_000u128;
        let timezone = Timezone::hour_shift(3);
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        context.switch_account_to_ft_contract_account();
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(amount_per_receiver * 2),
            airdrop_msg_with_timezone(&product.id, &[alice.clone(), bob.clone()], Some(timezone)),
        );

        let contract = context.contract();
        assert_eq!(timezone, contract.get_account(&alice).timezone);
        assert_eq!(timezone, contract.get_account(&bob).timezone);
    }

    #[rstest]
    fn airdrop_score_based_preserves_existing_timezone(
        admin: AccountId,
        alice: AccountId,
        product_1_year_12_cap_score_based: Product,
    ) {
        let product = product_1_year_12_cap_score_based;
        let amount_per_receiver = 1_000_000u128;
        let original_timezone = Timezone::hour_shift(5);
        let airdrop_timezone = Timezone::hour_shift(3);
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        // First airdrop sets alice's timezone to +5
        context.switch_account_to_ft_contract_account();
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(amount_per_receiver),
            airdrop_msg_with_timezone(&product.id, &[alice.clone()], Some(original_timezone)),
        );

        // Second airdrop with +3 should not overwrite alice's timezone
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(amount_per_receiver),
            airdrop_msg_with_timezone(&product.id, &[alice.clone()], Some(airdrop_timezone)),
        );

        assert_eq!(original_timezone, context.contract().get_account(&alice).timezone);
    }

    #[rstest]
    fn airdrop_protected_product(
        admin: AccountId,
        alice: AccountId,
        bob: AccountId,
        #[from(protected_product)] ProtectedProduct { product, signer }: ProtectedProduct,
    ) {
        let amount_per_receiver = 1_000_000u128;
        let valid_until = 100_000_000u64;
        let receivers = vec![alice.clone(), bob.clone()];
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        let airdrop_message = AirdropMessage::new(
            &context.owner,
            &product.id,
            amount_per_receiver,
            &receivers,
            valid_until,
        );
        let signature: Base64String = signer.sign(airdrop_message.material()).into();

        let msg = json!({
            "type": "airdrop",
            "data": {
                "ticket": {
                    "product_id": product.id,
                    "valid_until": valid_until.to_string(),
                },
                "signature": *signature,
                "receivers": receivers,
            }
        });

        context.switch_account_to_ft_contract_account();
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(amount_per_receiver * 2),
            msg.to_string(),
        );

        let contract = context.contract();
        assert_eq!(amount_per_receiver, contract.get_account(&alice).get_jar(&product.id).total_principal());
        assert_eq!(amount_per_receiver, contract.get_account(&bob).get_jar(&product.id).total_principal());
    }

    #[rstest]
    #[should_panic(expected = "Only manager can perform airdrops")]
    fn airdrop_not_manager(admin: AccountId, alice: AccountId, bob: AccountId, product: Product) {
        let amount_per_receiver = 1_000_000u128;
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        context.switch_account_to_ft_contract_account();
        // alice is not the manager
        context.contract().ft_on_transfer(
            alice.clone(),
            U128(amount_per_receiver * 2),
            airdrop_msg(&product.id, &[alice.clone(), bob.clone()]),
        );
    }

    #[rstest]
    #[should_panic(expected = "Amount must be evenly divisible among receivers")]
    fn airdrop_indivisible_amount(admin: AccountId, alice: AccountId, bob: AccountId, product: Product) {
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        context.switch_account_to_ft_contract_account();
        // 3 tokens for 2 receivers — not divisible
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(3),
            airdrop_msg(&product.id, &[alice.clone(), bob.clone()]),
        );
    }

    #[rstest]
    #[should_panic(expected = "Receivers list is empty")]
    fn airdrop_empty_receivers(admin: AccountId, product: Product) {
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);

        context.switch_account_to_ft_contract_account();
        context.contract().ft_on_transfer(
            admin.clone(),
            U128(1_000_000),
            airdrop_msg(&product.id, &[]),
        );
    }

    #[rstest]
    #[should_panic(expected = "Ticket is outdated")]
    fn airdrop_expired_ticket(
        admin: AccountId,
        alice: AccountId,
        #[from(protected_product)] ProtectedProduct { product, signer }: ProtectedProduct,
    ) {
        let amount_per_receiver = 1_000_000u128;
        let valid_until = 1u64; // already expired
        let receivers = vec![alice.clone()];
        let mut context = Context::new(admin.clone()).with_products(&[product.clone()]);
        context.set_block_timestamp_in_ms(1_000); // advance past valid_until

        let airdrop_message = AirdropMessage::new(
            &context.owner,
            &product.id,
            amount_per_receiver,
            &receivers,
            valid_until,
        );
        let signature: Base64String = signer.sign(airdrop_message.material()).into();

        let msg = json!({
            "type": "airdrop",
            "data": {
                "ticket": {
                    "product_id": product.id,
                    "valid_until": valid_until.to_string(),
                },
                "signature": *signature,
                "receivers": receivers,
            }
        });

        context.switch_account_to_ft_contract_account();
        context.contract().ft_on_transfer(admin.clone(), U128(amount_per_receiver), msg.to_string());
    }
}
