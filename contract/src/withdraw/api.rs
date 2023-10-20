use model::{
    jar::{JarId, JarIdView},
    withdraw::{Fee, WithdrawView},
    TokenAmount,
};
use near_sdk::{ext_contract, is_promise_success, json_types::U128, near_bindgen, PromiseOrValue};

use crate::{
    assert::{assert_is_liquidable, assert_not_locked, assert_sufficient_balance},
    env,
    event::{emit, EventKind, WithdrawData},
    jar::model::Jar,
    product::model::WithdrawalFee,
    AccountId, Contract, ContractExt, Product,
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
        account_id: AccountId,
        jar_id: JarId,
        close_jar: bool,
        withdrawn_amount: TokenAmount,
        fee: Option<Fee>,
    ) -> WithdrawView;
}

#[near_bindgen]
impl WithdrawApi for Contract {
    fn withdraw(&mut self, jar_id: JarIdView, amount: Option<U128>) -> PromiseOrValue<WithdrawView> {
        let account_id = env::predecessor_account_id();
        let jar = self.get_jar_internal(&account_id, jar_id.0).clone();

        assert_not_locked(&jar);

        let amount = amount.map_or(jar.principal, |value| value.0);

        assert_sufficient_balance(&jar, amount);

        let now = env::block_timestamp_ms();
        let product = self.get_product(&jar.product_id);

        assert_is_liquidable(&jar, product, now);

        let mut withdrawn_jar = jar.withdrawn(product, amount, now);
        let close_jar = withdrawn_jar.should_be_closed(product, now);

        withdrawn_jar.lock();
        *self.get_jar_mut_internal(&jar.account_id, jar.id) = withdrawn_jar;

        self.transfer_withdraw(&account_id, amount, &jar, close_jar)
    }
}

impl Contract {
    pub(crate) fn after_withdraw_internal(
        &mut self,
        account_id: AccountId,
        jar_id: JarId,
        close_jar: bool,
        withdrawn_amount: TokenAmount,
        fee: Option<Fee>,
        is_promise_success: bool,
    ) -> WithdrawView {
        if !is_promise_success {
            let jar = self.get_jar_mut_internal(&account_id, jar_id);
            jar.principal += withdrawn_amount;
            jar.unlock();

            return WithdrawView::new(0, None);
        }

        if close_jar {
            self.delete_jar(&account_id, jar_id);
        } else {
            self.get_jar_mut_internal(&account_id, jar_id).unlock();
        }

        let withdrawal_result = WithdrawView::new(withdrawn_amount, fee);

        emit(EventKind::Withdraw(WithdrawData {
            id: jar_id,
            withdrawn_amount: withdrawal_result.withdrawn_amount,
            fee_amount: withdrawal_result.fee,
        }));

        withdrawal_result
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
        close_jar: bool,
    ) -> PromiseOrValue<WithdrawView> {
        let product = self.get_product(&jar.product_id);
        let fee = self.get_fee(product, jar);

        self.ft_contract()
            .transfer(account_id, amount, "withdraw", &fee)
            .then(Self::after_withdraw_call(
                account_id.clone(),
                jar.id,
                close_jar,
                amount,
                &fee,
            ))
            .into()
    }

    fn after_withdraw_call(
        account_id: AccountId,
        jar_id: JarId,
        close_jar: bool,
        withdrawn_balance: TokenAmount,
        fee: &Option<Fee>,
    ) -> Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_WITHDRAW)
            .after_withdraw(account_id, jar_id, close_jar, withdrawn_balance, fee.clone())
    }
}

#[cfg(test)]
impl Contract {
    fn transfer_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: TokenAmount,
        jar: &Jar,
        close_jar: bool,
    ) -> PromiseOrValue<WithdrawView> {
        let product = self.get_product(&jar.product_id);
        let fee = self.get_fee(product, jar);

        let withdrawn = self.after_withdraw_internal(
            account_id.clone(),
            jar.id,
            close_jar,
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
        account_id: AccountId,
        jar_id: JarId,
        close_jar: bool,
        withdrawn_amount: TokenAmount,
        fee: Option<Fee>,
    ) -> WithdrawView {
        self.after_withdraw_internal(
            account_id,
            jar_id,
            close_jar,
            withdrawn_amount,
            fee,
            is_promise_success(),
        )
    }
}
