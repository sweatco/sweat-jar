#[cfg(test)]
use common::test_data::get_test_future_success;
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

#[cfg(not(test))]
use crate::internal::assert_gas;
use crate::internal::is_promise_success;

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub(super) struct WithdrawalRequest {
    pub product_id: ProductId,
    pub amount: TokenAmount,
    pub fee: TokenAmount,
    pub partition_index: usize,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub(super) struct BulkWithdrawalRequest {
    pub requests: Vec<WithdrawalRequest>,
    pub total_amount: TokenAmount,
    pub total_fee: TokenAmount,
}

#[cfg(not(test))]
use crate::common::gas_data::{GAS_FOR_BULK_AFTER_WITHDRAW, GAS_FOR_FT_TRANSFER};
#[cfg(not(test))]
use crate::ft_interface::FungibleTokenInterface;
use crate::{
    common, env,
    event::{emit, EventKind, WithdrawData},
    AccountId, Contract, ContractExt,
};

#[allow(dead_code)] // False positive since rust 1.78. It is used from `ext_contract` macro.
#[ext_contract(ext_self)]
pub(super) trait WithdrawCallbacks {
    fn after_withdraw(&mut self, account_id: AccountId, request: WithdrawalRequest) -> WithdrawView;

    fn after_bulk_withdraw(&mut self, account_id: AccountId, request: BulkWithdrawalRequest) -> BulkWithdrawView;
}

#[near_bindgen]
impl WithdrawApi for Contract {
    // TODO: doc change
    fn withdraw(&mut self, product_id: ProductId) -> PromiseOrValue<WithdrawView> {
        let account_id = env::predecessor_account_id();
        self.assert_migrated(&account_id);

        self.get_account_mut(&account_id).get_jar_mut(&product_id).try_lock();
        self.update_jar_cache(&account_id, &product_id);

        let jar = self.get_account(&account_id).get_jar(&product_id);
        let product = self.get_product(&product_id);
        let (amount, partition_index) = jar.get_liquid_balance(&product.terms);
        let fee = product.calculate_fee(amount);

        let request = WithdrawalRequest {
            product_id,
            amount,
            fee,
            partition_index,
        };

        self.transfer_withdraw(&account_id, request)
    }

    fn withdraw_all(&mut self) -> PromiseOrValue<BulkWithdrawView> {
        let account_id = env::predecessor_account_id();
        self.assert_migrated(&account_id);

        self.update_account_cache(&account_id, None);

        let mut request = BulkWithdrawalRequest::default();

        for (product_id, jar) in &self.get_account(&account_id).jars {
            if jar.is_pending_withdraw {
                continue;
            }

            let product = self.get_product(product_id);
            let (amount, partition_index) = jar.get_liquid_balance(&product.terms);
            let fee = product.calculate_fee(amount);

            request.requests.push(WithdrawalRequest {
                amount,
                partition_index,
                product_id: product.id,
                fee,
            });
            request.total_amount += amount;
            request.total_fee += fee;
        }

        for request in &request.requests {
            self.get_account_mut(&account_id)
                .get_jar_mut(&request.product_id)
                .lock();
        }

        if request.requests.is_empty() {
            return PromiseOrValue::Value(BulkWithdrawView::default());
        }

        self.transfer_bulk_withdraw(&account_id, request)
    }
}

impl Contract {
    pub(super) fn after_withdraw_internal(
        &mut self,
        account_id: AccountId,
        request: WithdrawalRequest,
        is_promise_success: bool,
    ) -> WithdrawView {
        let account = self.get_account_mut(&account_id);
        account.get_jar_mut(&request.product_id).unlock();

        if !is_promise_success {
            return WithdrawView::new(&request.product_id, 0, None);
        }

        self.clean_up(&account_id, &request);

        let withdrawal_result = WithdrawView::new(&request.product_id, request.amount, self.wrap_fee(request.fee));

        emit(EventKind::Withdraw((
            request.product_id,
            withdrawal_result.fee,
            withdrawal_result.withdrawn_amount,
        )));

        withdrawal_result
    }

    pub(super) fn after_bulk_withdraw_internal(
        &mut self,
        account_id: AccountId,
        request: BulkWithdrawalRequest,
        is_promise_success: bool,
    ) -> BulkWithdrawView {
        if !is_promise_success {
            return self.process_bulk_withdrawal_error(&account_id, request);
        }

        let result = self.process_bulk_withdrawal_success(&account_id, request);
        emit(collect_bulk_withdrawal_event_data(&result));

        result
    }

    fn process_bulk_withdrawal_error(
        &mut self,
        account_id: &AccountId,
        request: BulkWithdrawalRequest,
    ) -> BulkWithdrawView {
        let account = self.get_account_mut(account_id);
        for request in request.requests {
            let jar = account.get_jar_mut(&request.product_id);
            jar.unlock();
        }

        BulkWithdrawView::default()
    }

    fn process_bulk_withdrawal_success(
        &mut self,
        account_id: &AccountId,
        request: BulkWithdrawalRequest,
    ) -> BulkWithdrawView {
        let mut result = BulkWithdrawView::default();

        for request in &request.requests {
            self.get_account_mut(account_id)
                .get_jar_mut(&request.product_id)
                .unlock();

            let deposit_withdrawal = WithdrawView::new(&request.product_id, request.amount, self.wrap_fee(request.fee));

            result.total_amount.0 += deposit_withdrawal.withdrawn_amount.0;
            result.withdrawals.push(deposit_withdrawal);
        }

        for request in &request.requests {
            self.clean_up(account_id, request);
        }

        result
    }

    pub(crate) fn wrap_fee(&self, amount: TokenAmount) -> Option<Fee> {
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

fn collect_bulk_withdrawal_event_data(withdrawal_result: &BulkWithdrawView) -> EventKind {
    let event_data: Vec<WithdrawData> = withdrawal_result
        .withdrawals
        .iter()
        .map(|withdrawal| {
            (
                withdrawal.product_id.clone(),
                withdrawal.fee,
                withdrawal.withdrawn_amount,
            )
        })
        .collect();

    EventKind::WithdrawAll(event_data)
}

impl Contract {
    fn clean_up(&mut self, account_id: &AccountId, request: &WithdrawalRequest) {
        let jar = self.get_account_mut(account_id).get_jar_mut(&request.product_id);
        jar.clean_up_deposits(request.partition_index);

        let jar = self.get_account(account_id).get_jar(&request.product_id);
        if jar.should_close() {
            self.get_account_mut(account_id).jars.remove(&request.product_id);
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
        let withdrawn = self.after_withdraw_internal(account_id.clone(), request, get_test_future_success());

        PromiseOrValue::Value(withdrawn)
    }

    fn transfer_bulk_withdraw(
        &mut self,
        account_id: &AccountId,
        request: BulkWithdrawalRequest,
    ) -> PromiseOrValue<BulkWithdrawView> {
        let withdrawn = self.after_bulk_withdraw_internal(account_id.clone(), request, get_test_future_success());

        PromiseOrValue::Value(withdrawn)
    }
}

#[near_bindgen]
#[mutants::skip] // Covered by integration tests
impl WithdrawCallbacks for Contract {
    #[private]
    fn after_withdraw(&mut self, account_id: AccountId, request: WithdrawalRequest) -> WithdrawView {
        self.after_withdraw_internal(account_id, request, is_promise_success())
    }

    #[private]
    fn after_bulk_withdraw(&mut self, account_id: AccountId, request: BulkWithdrawalRequest) -> BulkWithdrawView {
        self.after_bulk_withdraw_internal(account_id, request, is_promise_success())
    }
}
