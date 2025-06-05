use std::collections::HashMap;

use near_sdk::{env, ext_contract, json_types::U128, near, AccountId, PromiseOrValue};
use sweat_jar_model::{
    api::ClaimApi,
    data::{
        account::v1::AccountV1Companion, claim::ClaimedAmountView, jar::AggregatedTokenAmountView, product::ProductId,
    },
    interest::InterestCalculator,
    TokenAmount,
};

#[cfg(not(test))]
use crate::{common::assertions::assert_gas, feature::ft_interface::FungibleTokenInterface};
use crate::{
    common::{
        env::env_ext,
        event::{emit, ClaimData, EventKind},
    },
    Contract, ContractExt,
};

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
mod gas {
    use near_sdk::Gas;

    /// Const of after claim call with 1 jar
    pub(super) const INITIAL_GAS_FOR_AFTER_CLAIM: Gas = Gas::from_tgas(4);

    /// Cost of adding 1 additional jar in after claim call. Measured with `measure_after_claim_total_test`
    pub(super) const ADDITIONAL_AFTER_CLAIM_JAR_COST: Gas = Gas::from_ggas(80);

    /// Values are measured with `measure_after_claim_total_test`
    /// For now number of jars is arbitrary
    pub(super) const GAS_FOR_AFTER_CLAIM: Gas =
        Gas::from_gas(INITIAL_GAS_FOR_AFTER_CLAIM.as_gas() + ADDITIONAL_AFTER_CLAIM_JAR_COST.as_gas() * 200);
}

#[allow(dead_code)] // False positive since rust 1.78. It is used from `ext_contract` macro.
#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(
        &mut self,
        account_id: AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> ClaimedAmountView;
}

#[near]
impl ClaimApi for Contract {
    fn claim_total(&mut self, detailed: Option<bool>) -> PromiseOrValue<ClaimedAmountView> {
        let account_id = env::predecessor_account_id();

        let account = self.get_account(&account_id);
        let mut accumulator = ClaimedAmountView::new(detailed);
        let now = env::block_timestamp_ms();

        let mut rollback_jars = HashMap::new();
        let mut interest_per_jar: HashMap<ProductId, (TokenAmount, u64)> = HashMap::new();
        let mut event_data = ClaimData::new(now);

        for (product_id, jar) in &account.jars {
            if jar.is_pending_withdraw {
                continue;
            }

            rollback_jars.insert(product_id.clone(), jar.to_rollback());

            let product = self.get_product(product_id);
            let (interest, remainder) = product.terms.get_interest(account, jar, now);

            if interest == 0 {
                continue;
            }

            interest_per_jar.insert(product_id.clone(), (interest, remainder));
            accumulator.add(product_id, interest);
        }

        let account = self.get_account_mut(&account_id);
        for (product_id, (interest, remainder)) in interest_per_jar {
            let jar = account.get_jar_mut(&product_id);
            jar.claim(remainder, now).lock();

            event_data.add((product_id.clone(), interest.into()));
        }

        let account_rollback = AccountV1Companion {
            score: account.score.into(),
            jars: rollback_jars.into(),
            ..AccountV1Companion::default()
        };

        account.score.try_reset_score();

        if accumulator.get_total().0 > 0 {
            self.claim_interest(
                &account_id,
                accumulator,
                account_rollback,
                EventKind::Claim(account_id.clone(), event_data),
            )
        } else {
            PromiseOrValue::Value(accumulator)
        }
    }
}

impl Contract {
    #[cfg(test)]
    fn claim_interest(
        &mut self,
        account_id: &AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        use crate::common::env::env_ext;

        PromiseOrValue::Value(self.after_claim_internal(
            account_id.clone(),
            claimed_amount,
            account_rollback,
            event,
            env_ext::is_promise_success(),
        ))
    }

    #[cfg(not(test))]
    #[mutants::skip] // Covered by integration tests
    fn claim_interest(
        &mut self,
        account_id: &AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        use crate::feature::ft_interface::gas::GAS_FOR_FT_TRANSFER;

        assert_gas(
            GAS_FOR_FT_TRANSFER.as_gas() * 2 + gas::GAS_FOR_AFTER_CLAIM.as_gas(),
            || "Not enough gas for claim".to_string(),
        );

        self.ft_contract()
            .ft_transfer(account_id, claimed_amount.get_total().0, "claim")
            .then(after_claim_call(
                account_id.clone(),
                claimed_amount,
                account_rollback,
                event,
            ))
            .into()
    }

    fn after_claim_internal(
        &mut self,
        account_id: AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
        is_promise_success: bool,
    ) -> ClaimedAmountView {
        if is_promise_success {
            let account = self.accounts.get_mut(&account_id).expect("Account is not found");
            let jars = account_rollback.jars.expect("Jars are required in rollback account");

            for (product_id, _) in jars {
                let jar = account.get_jar_mut(&product_id);
                jar.unlock();

                if jar.should_close() {
                    account.jars.remove(&product_id);
                }
            }

            emit(event);

            claimed_amount
        } else {
            let account = self.get_account_mut(&account_id);
            account.apply(&account_rollback);

            match claimed_amount {
                ClaimedAmountView::Total(_) => ClaimedAmountView::Total(U128(0)),
                ClaimedAmountView::Detailed(_) => ClaimedAmountView::Detailed(AggregatedTokenAmountView::default()),
            }
        }
    }
}

#[near]
impl ClaimCallbacks for Contract {
    #[private]
    fn after_claim(
        &mut self,
        account_id: AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> ClaimedAmountView {
        self.after_claim_internal(
            account_id,
            claimed_amount,
            account_rollback,
            event,
            env_ext::is_promise_success(),
        )
    }
}

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
fn after_claim_call(
    account_id: AccountId,
    claimed_amount: ClaimedAmountView,
    account_rollback: AccountV1Companion,
    event: EventKind,
) -> near_sdk::Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(gas::GAS_FOR_AFTER_CLAIM)
        .after_claim(account_id, claimed_amount, account_rollback, event)
}
