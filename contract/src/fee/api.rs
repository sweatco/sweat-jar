use std::convert::Into;

#[cfg(not(test))]
use near_sdk::env;
use near_sdk::{ext_contract, json_types::U128, near, PromiseOrValue};
use sweat_jar_model::{api::FeeApi, TokenAmount};

#[cfg(not(test))]
use crate::{common::gas_data::GAS_FOR_AFTER_FEE_WITHDRAW, ft_interface::FungibleTokenInterface};
use crate::{internal::is_promise_success, Contract, ContractExt};

#[near]
impl FeeApi for Contract {
    fn get_fee_amount(&self) -> U128 {
        self.fee_amount.into()
    }

    fn withdraw_fee(&mut self) -> PromiseOrValue<U128> {
        self.assert_manager();

        let amount = self.fee_amount;
        self.fee_amount = 0;

        self.withdraw_fee_internal(amount)
    }
}

#[cfg(not(test))]
impl Contract {
    fn withdraw_fee_internal(&mut self, amount: TokenAmount) -> PromiseOrValue<U128> {
        self.ft_contract()
            .ft_transfer(&self.fee_account_id, amount, "withdraw_fee")
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_AFTER_FEE_WITHDRAW)
                    .after_fee_withdrawn(amount.into()),
            )
            .into()
    }
}

#[cfg(test)]
impl Contract {
    fn withdraw_fee_internal(&mut self, amount: TokenAmount) -> PromiseOrValue<U128> {
        PromiseOrValue::Value(self.after_fee_withdrawn(amount.into()))
    }
}

#[ext_contract(ext_self)]
trait FeeWithdrawCallback {
    fn after_fee_withdrawn(&mut self, amount: U128) -> U128;
}

#[near]
impl FeeWithdrawCallback for Contract {
    #[private]
    fn after_fee_withdrawn(&mut self, amount: U128) -> U128 {
        if is_promise_success() {
            return amount;
        }

        self.fee_amount += amount.0;

        0.into()
    }
}
