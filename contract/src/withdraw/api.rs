use common::test_data;
use near_sdk::{
    ext_contract, near_bindgen,
    serde::{Deserialize, Serialize},
    PromiseOrValue,
};
use sweat_jar_model::{
    api::WithdrawApi,
    withdraw::{BulkWithdrawView, Fee, WithdrawView},
    ProductId, TokenAmount,
};

use crate::internal::{assert_gas, is_promise_success};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct WithdrawalRequest {
    pub product_id: ProductId,
    pub amount: TokenAmount,
    pub fee: TokenAmount,
    pub partition_index: usize,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct BulkWithdrawalRequest {
    pub requests: Vec<WithdrawalRequest>,
    pub total_amount: TokenAmount,
    pub total_fee: TokenAmount,
}

#[cfg(not(test))]
use crate::ft_interface::FungibleTokenInterface;
use crate::{
    assert::assert_not_locked,
    common,
    common::{
        gas_data::{GAS_FOR_BULK_AFTER_WITHDRAW, GAS_FOR_FT_TRANSFER},
        Timestamp,
    },
    env,
    event::{emit, EventKind},
    jar::model::JarV2,
    product::model::v2::Terms,
    AccountId, Contract, ContractExt,
};

#[allow(dead_code)] // False positive since rust 1.78. It is used from `ext_contract` macro.
#[ext_contract(ext_self)]
pub trait WithdrawCallbacks {
    fn after_withdraw(&mut self, account_id: AccountId, request: WithdrawalRequest) -> WithdrawView;

    fn after_bulk_withdraw(&mut self, account_id: AccountId, request: BulkWithdrawalRequest) -> BulkWithdrawView;
}

#[near_bindgen]
impl WithdrawApi for Contract {
    // TODO: doc change
    fn withdraw(&mut self, product_id: ProductId) -> PromiseOrValue<WithdrawView> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_if_needed(&account_id);

        let account = self.get_account_mut(&account_id);

        let jar = account.jars.get_mut(&product_id).expect("No jar for the product");
        assert_not_locked(jar);
        jar.lock();

        // TODO: add method for withdrawal on a single jar
        self.update_cache(account);

        let product = self.get_product(&product_id);
        let (amount, partition_index) = jar.get_liquid_balance(product.terms, env::block_timestamp_ms());
        let fee = product.calculate_fee(amount);

        let request = WithdrawalRequest {
            amount,
            partition_index,
            product_id,
            fee,
        };

        self.transfer_withdraw(&account_id, request)
    }

    fn withdraw_all(&mut self) -> PromiseOrValue<BulkWithdrawView> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_if_needed(&account_id);
        let now = env::block_timestamp_ms();

        let account = self.get_account_mut(&account_id);
        self.update_cache(account);

        let request: BulkWithdrawalRequest = account
            .jars
            .iter_mut()
            .filter(|(_, jar)| !jar.is_pending_withdraw)
            .fold(
                BulkWithdrawalRequest::default(),
                |acc: &mut BulkWithdrawalRequest, (product_id, jar)| {
                    let product = self.get_product(&product_id);
                    jar.lock();

                    let (amount, partition_index) = jar.get_liquid_balance(product.terms, now);
                    let fee = product.calculate_fee(amount);

                    acc.requests.push(WithdrawalRequest {
                        amount,
                        partition_index,
                        product_id,
                        fee,
                    });
                    acc.total_amount += amount;
                    acc.total_fee += fee;
                },
            );

        if request.requests.is_empty() {
            return PromiseOrValue::Value(BulkWithdrawView::default());
        }

        self.transfer_bulk_withdraw(&account_id, request)
    }
}

impl Contract {
    pub(crate) fn after_withdraw_internal(
        &mut self,
        account_id: AccountId,
        request: WithdrawalRequest,
        is_promise_success: bool,
    ) -> WithdrawView {
        let account = self.get_account_mut(&account_id);
        let jar = account
            .jars
            .get_mut(&request.product_id)
            .expect("No jar found for the product");
        jar.unlock();

        if !is_promise_success {
            return WithdrawView::new(0, None);
        }

        if jar.deposits.len() == request.partition_index {
            jar.deposits.clear();
        } else {
            jar.deposits.drain(..request.partition_index);
        }

        if jar.should_close() {
            account.jars.remove(&request.product_id);
        }

        let withdrawal_result = WithdrawView::new(request.amount, self.wrap_fee(request.fee));

        emit(EventKind::Withdraw((
            request.product_id,
            withdrawal_result.fee,
            withdrawal_result.withdrawn_amount,
        )));

        withdrawal_result
    }

    pub(crate) fn after_bulk_withdraw_internal(
        &mut self,
        account_id: AccountId,
        request: BulkWithdrawalRequest,
        is_promise_success: bool,
    ) -> BulkWithdrawView {
        let account = self.get_account_mut(&account_id);

        let mut withdrawal_result = BulkWithdrawView {
            total_amount: 0.into(),
            withdrawals: vec![],
        };

        if !is_promise_success {
            for request in request.requests {
                let jar = account
                    .jars
                    .get_mut(&request.product_id)
                    .expect("No jar for the product");
                jar.unlock();
            }

            return withdrawal_result;
        }

        let mut event_data = vec![];

        for request in request.requests {
            let jar = account
                .jars
                .get_mut(&request.product_id)
                .expect("No jar found for the product");

            jar.unlock();

            if jar.deposits.len() == request.partition_index {
                jar.deposits.clear();
            } else {
                jar.deposits.drain(..request.partition_index);
            }

            if jar.should_close() {
                account.jars.remove(&request.product_id);
            }

            let deposit_withdrawal = WithdrawView::new(request.amount, self.wrap_fee(request.fee));

            event_data.push((
                request.product_id,
                deposit_withdrawal.fee,
                deposit_withdrawal.withdrawn_amount,
            ));

            withdrawal_result.total_amount.0 += deposit_withdrawal.withdrawn_amount.0;
            withdrawal_result.withdrawals.push(deposit_withdrawal);
        }

        emit(EventKind::WithdrawAll(event_data));

        withdrawal_result
    }

    fn wrap_fee(&self, amount: TokenAmount) -> Option<Fee> {
        if amount == 0 {
            None
        } else {
            Some(Fee {
                beneficiary_id: self.fee_account_id.clone(),
                amount,
            })
        }
    }
}

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
impl Contract {
    fn transfer_withdraw(
        &mut self,
        account_id: &AccountId,
        request: WithdrawalRequest,
    ) -> PromiseOrValue<WithdrawView> {
        self.ft_contract()
            .ft_transfer(account_id, request.amount, "withdraw", &self.wrap_fee(request.fee))
            .then(Self::after_withdraw_call(account_id.clone(), request))
            .into()
    }

    fn transfer_bulk_withdraw(
        &mut self,
        account_id: &AccountId,
        request: BulkWithdrawalRequest,
    ) -> PromiseOrValue<BulkWithdrawView> {
        assert_gas(
            GAS_FOR_FT_TRANSFER.as_gas() + GAS_FOR_BULK_AFTER_WITHDRAW.as_gas(),
            || "Not enough gas to finish withdrawal",
        );

        self.ft_contract()
            .ft_transfer(
                account_id,
                request.total_amount,
                "bulk_withdraw",
                &self.wrap_fee(request.total_fee),
            )
            .then(Self::after_bulk_withdraw_call(account_id.clone(), request))
            .into()
    }

    fn after_withdraw_call(account_id: AccountId, request: WithdrawalRequest) -> near_sdk::Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(common::gas_data::GAS_FOR_AFTER_WITHDRAW)
            .after_withdraw(account_id, request)
    }

    fn after_bulk_withdraw_call(account_id: AccountId, request: BulkWithdrawalRequest) -> near_sdk::Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(GAS_FOR_BULK_AFTER_WITHDRAW)
            .after_bulk_withdraw(account_id, request)
    }
}

#[cfg(test)]
impl Contract {
    fn transfer_withdraw(
        &mut self,
        account_id: &AccountId,
        request: WithdrawalRequest,
    ) -> PromiseOrValue<WithdrawView> {
        let withdrawn = self.after_withdraw_internal(account_id.clone(), request, test_data::get_test_future_success());

        PromiseOrValue::Value(withdrawn)
    }

    fn transfer_bulk_withdraw(
        &mut self,
        account_id: &AccountId,
        request: BulkWithdrawalRequest,
    ) -> PromiseOrValue<BulkWithdrawView> {
        let withdrawn =
            self.after_bulk_withdraw_internal(account_id.clone(), request, test_data::get_test_future_success());

        PromiseOrValue::Value(withdrawn)
    }
}

impl JarV2 {
    fn get_liquid_balance(&self, terms: &Terms, now: Timestamp) -> (TokenAmount, usize) {
        if terms.allows_early_withdrawal() {
            let sum = self.deposits.iter().map(|deposit| deposit.principal).sum();
            let partition_index = self.deposits.len();

            (sum, partition_index)
        } else {
            let partition_index = self.deposits.partition_point(|deposit| deposit.is_liquid(now, todo!()));

            let sum = self.deposits[..partition_index]
                .iter()
                .map(|deposit| deposit.principal)
                .sum();

            (sum, partition_index)
        }
    }

    fn should_close(&self) -> bool {
        self.deposits.is_empty() && self.cache.map_or(true, |cache| cache.interest == 0)
    }
}

#[near_bindgen]
#[mutants::skip] // Covered by integration tests
impl WithdrawCallbacks for Contract {
    #[private]
    fn after_withdraw(&mut self, account_id: AccountId, request: WithdrawalRequest) -> WithdrawView {
        self.after_withdraw_internal(account_id, request)
    }

    #[private]
    fn after_bulk_withdraw(&mut self, account_id: AccountId, request: BulkWithdrawalRequest) -> BulkWithdrawView {
        self.after_bulk_withdraw_internal(account_id, request, is_promise_success())
    }
}
