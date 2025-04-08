use std::collections::HashSet;

#[cfg(test)]
use common::test_data::get_test_future_success;
use near_sdk::{
    env::panic_str,
    ext_contract, near, near_bindgen,
    serde::{Deserialize, Serialize},
    PromiseOrValue,
};
use sweat_jar_model::{
    api::WithdrawApi,
    withdraw::{BulkWithdrawView, WithdrawView},
    ProductId, TokenAmount,
};

#[cfg(not(test))]
use crate::internal::assert_gas;
use crate::{internal::is_promise_success, jar::model::Jar};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub(crate) struct WithdrawalRequest {
    pub product_id: ProductId,
    pub withdrawal: WithdrawalDto,
    pub partition_index: usize,
}

#[derive(Debug, Default, Copy, Clone)]
#[near(serializers=[json])]
pub(crate) struct WithdrawalDto {
    pub amount: TokenAmount,
    pub fee: TokenAmount,
}

impl WithdrawalDto {
    pub fn new(amount: TokenAmount, fee: TokenAmount) -> Self {
        Self { amount, fee }
    }

    #[cfg(not(test))]
    pub fn net_amount(&self) -> TokenAmount {
        self.amount - self.fee
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub(super) struct BulkWithdrawalRequest {
    pub requests: Vec<WithdrawalRequest>,
}

#[cfg(not(test))]
impl BulkWithdrawalRequest {
    fn total_net_amount(&self) -> TokenAmount {
        self.requests
            .iter()
            .map(|request| request.withdrawal.net_amount())
            .sum()
    }
}

#[cfg(not(test))]
use crate::common::gas_data::{GAS_FOR_BULK_AFTER_WITHDRAW, GAS_FOR_FT_TRANSFER};
#[cfg(not(test))]
use crate::ft_interface::FungibleTokenInterface;
use crate::{
    common, env,
    event::{emit, EventKind, WithdrawData},
    product::model::v1::ProductModelApi,
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
            withdrawal: WithdrawalDto::new(amount, fee),
            partition_index,
        };

        self.transfer_withdraw(&account_id, request)
    }

    fn withdraw_all(&mut self, product_ids: Option<HashSet<ProductId>>) -> PromiseOrValue<BulkWithdrawView> {
        let account_id = env::predecessor_account_id();
        self.assert_migrated(&account_id);

        self.update_account_cache(&account_id, None);

        let mut request = BulkWithdrawalRequest::default();

        let product_ids = product_ids.unwrap_or_else(|| self.get_account(&account_id).jars.keys().cloned().collect());
        let account = self.get_account(&account_id);

        for product_id in product_ids {
            let jar = account
                .jars
                .get(&product_id)
                .unwrap_or_else(|| panic_str(&format!("No jar found for {product_id}")));
            if jar.is_pending_withdraw {
                continue;
            }

            let product = self.get_product(&product_id);
            let (amount, partition_index) = jar.get_liquid_balance(&product.terms);
            let fee = product.calculate_fee(amount);

            request.requests.push(WithdrawalRequest {
                product_id: product.id,
                withdrawal: WithdrawalDto::new(amount, fee),
                partition_index,
            });
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
            return WithdrawView::new(&request.product_id, 0, 0);
        }

        self.clean_up(&account_id, &request);
        self.fee_amount += request.withdrawal.fee;

        let withdrawal_result =
            WithdrawView::new(&request.product_id, request.withdrawal.amount, request.withdrawal.fee);

        emit(EventKind::Withdraw(
            account_id,
            (
                request.product_id,
                withdrawal_result.fee,
                withdrawal_result.withdrawn_amount,
            ),
        ));

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
        emit(collect_bulk_withdrawal_event_data(account_id, &result));

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

            let deposit_withdrawal =
                WithdrawView::new(&request.product_id, request.withdrawal.amount, request.withdrawal.fee);

            result.total_amount.0 += deposit_withdrawal.withdrawn_amount.0;
            result.withdrawals.push(deposit_withdrawal);
        }

        for request in &request.requests {
            self.fee_amount += request.withdrawal.fee;
            self.clean_up(account_id, request);
        }

        result
    }
}

fn collect_bulk_withdrawal_event_data(account_id: AccountId, withdrawal_result: &BulkWithdrawView) -> EventKind {
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

    EventKind::WithdrawAll(account_id, event_data)
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
            .ft_transfer(account_id, request.withdrawal.net_amount(), "withdraw")
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
            .ft_transfer(account_id, request.total_net_amount(), "bulk_withdraw")
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
