use model::{jar_view::JarIdView, withdraw_view::WithdrawView, Fee, TokenAmount};
use near_sdk::{ext_contract, is_promise_success, json_types::U128, near_bindgen, PromiseOrValue};

use crate::{
    assert::{assert_is_liquidable, assert_sufficient_balance},
    assert_ownership, env,
    event::{emit, EventKind, WithdrawData},
    product::model::WithdrawalFee,
    AccountId, Contract, ContractExt, Jar, Product,
};
#[cfg(not(test))]
use crate::{ft_interface::FungibleTokenInterface, Promise};

/// The `WithdrawApi` trait defines methods for withdrawing tokens from specific deposit jars within the smart contract.
pub trait WithdrawApi {
    /// Allows the owner of a deposit jar to withdraw a specified amount of tokens from it.
    ///
    /// # Arguments
    ///
    /// * `jar_id` - The ID of the deposit jar from which the withdrawal is being made.
    /// * `amount` - An optional `U128` value indicating the amount of tokens to withdraw. If `None` is provided,
    ///              the entire balance of the jar will be withdrawn.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<WithdrawView>` which represents the result of the withdrawal. If the withdrawal is successful,
    /// it returns the withdrawn amount and probably fee, if it's defined by the associated Product.
    /// If there are insufficient funds or other conditions are not met, the contract might panic or
    /// return 0 for both withdrawn amount and fee.
    ///
    /// # Panics
    ///
    /// This function may panic under the following conditions:
    /// - If the caller is not the owner of the specified jar.
    /// - If the withdrawal amount exceeds the available balance in the jar.
    /// - If attempting to withdraw from a Fixed jar that is not yet mature.
    fn withdraw(&mut self, jar_id: JarIdView, amount: Option<U128>) -> PromiseOrValue<WithdrawView>;
}

#[ext_contract(ext_self)]
pub trait WithdrawCallbacks {
    fn after_withdraw(
        &mut self,
        jar_before_transfer: Jar,
        withdrawn_amount: TokenAmount,
        fee: Option<Fee>,
    ) -> WithdrawView;
}

#[near_bindgen]
impl WithdrawApi for Contract {
    fn withdraw(&mut self, jar_id: JarIdView, amount: Option<U128>) -> PromiseOrValue<WithdrawView> {
        let account_id = env::predecessor_account_id();
        let jar = self.get_jar_internal(&account_id, jar_id.0).locked();
        let amount = amount.map_or(jar.principal, |value| value.0);

        assert_ownership(&jar, &account_id);

        assert_sufficient_balance(&jar, amount);

        let now = env::block_timestamp_ms();
        let product = self.get_product(&jar.product_id);

        assert_is_liquidable(&jar, product, now);

        self.do_transfer(&account_id, &jar, amount)
    }
}

impl Contract {
    pub(crate) fn after_withdraw_internal(
        &mut self,
        jar_before_transfer: Jar,
        withdrawn_amount: TokenAmount,
        fee: Option<Fee>,
        is_promise_success: bool,
    ) -> WithdrawView {
        if is_promise_success {
            let product = self.get_product(&jar_before_transfer.product_id);
            let now = env::block_timestamp_ms();
            let (should_be_closed, jar) = jar_before_transfer.withdrawn(product, withdrawn_amount, now);

            let jar_id = jar.id;

            if should_be_closed {
                self.delete_jar(jar);
            } else {
                let stored_jar = self.get_jar_mut_internal(&jar.account_id, jar_id);
                *stored_jar = jar;
                stored_jar.unlock();
            }

            let withdrawal_result = WithdrawView::new(withdrawn_amount, fee);

            emit(EventKind::Withdraw(WithdrawData {
                id: jar_id,
                withdrawn_amount: withdrawal_result.withdrawn_amount,
                fee_amount: withdrawal_result.fee,
            }));

            withdrawal_result
        } else {
            let stored_jar = self.get_jar_mut_internal(&jar_before_transfer.account_id, jar_before_transfer.id);

            *stored_jar = jar_before_transfer.unlocked();

            WithdrawView::new(0, None)
        }
    }

    fn do_transfer(&mut self, account_id: &AccountId, jar: &Jar, amount: TokenAmount) -> PromiseOrValue<WithdrawView> {
        self.get_jar_mut_internal(account_id, jar.id).lock();
        self.transfer_withdraw(account_id, amount, jar)
    }

    fn get_fee(&self, product: &Product, jar: &Jar) -> Option<Fee> {
        let fee = product.withdrawal_fee.as_ref()?;

        let amount = match fee {
            WithdrawalFee::Fix(amount) => *amount,
            WithdrawalFee::Percent(percent) => percent * jar.principal,
        };

        Some(Fee {
            beneficiary_id: self.fee_account_id.clone(),
            amount,
        })
    }
}

#[cfg(not(test))]
impl Contract {
    fn transfer_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: TokenAmount,
        jar: &Jar,
    ) -> PromiseOrValue<WithdrawView> {
        let product = self.get_product(&jar.product_id);
        let fee = self.get_fee(product, jar);

        self.ft_contract()
            .transfer(account_id, amount, "withdraw", &fee)
            .then(Self::after_withdraw_call(jar.clone(), amount, &fee))
            .into()
    }

    fn after_withdraw_call(jar_before_transfer: Jar, withdrawn_balance: TokenAmount, fee: &Option<Fee>) -> Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_WITHDRAW)
            .after_withdraw(jar_before_transfer, withdrawn_balance, fee.clone())
    }
}

#[cfg(test)]
impl Contract {
    fn transfer_withdraw(&mut self, _: &AccountId, amount: TokenAmount, jar: &Jar) -> PromiseOrValue<WithdrawView> {
        let product = self.get_product(&jar.product_id);
        let fee = self.get_fee(product, jar);

        let withdrawn = self.after_withdraw_internal(
            jar.clone(),
            amount,
            fee,
            crate::common::test_data::get_test_future_success(),
        );

        PromiseOrValue::Value(withdrawn)
    }
}

#[near_bindgen]
impl WithdrawCallbacks for Contract {
    #[private]
    fn after_withdraw(
        &mut self,
        jar_before_transfer: Jar,
        withdrawn_amount: TokenAmount,
        fee: Option<Fee>,
    ) -> WithdrawView {
        self.after_withdraw_internal(jar_before_transfer, withdrawn_amount, fee, is_promise_success())
    }
}
