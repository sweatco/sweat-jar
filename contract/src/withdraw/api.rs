use near_sdk::{ext_contract, is_promise_success, near_bindgen, PromiseOrValue};
use near_sdk::json_types::{U128};

use crate::*;
use crate::assert::{assert_is_mature, assert_sufficient_balance};
use crate::common::TokenAmount;
use crate::event::{emit, EventKind, WithdrawData};
use crate::external::GAS_FOR_AFTER_TRANSFER;
use crate::ft_interface::Fee;
use crate::jar::model::JarIndex;
use crate::product::model::WithdrawalFee;

pub(crate) type WithdrawFunction = fn(
    contract: &mut Contract,
    account_id: &AccountId,
    amount: TokenAmount,
    jar: &Jar,
) -> PromiseOrValue<U128>;

pub trait WithdrawApi {
    fn withdraw(&mut self, jar_index: JarIndex, amount: Option<U128>) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_self)]
pub trait WithdrawCallbacks {
    fn after_withdraw(&mut self, jar_before_transfer: Jar, withdrawn_amount: TokenAmount) -> U128;
}

#[near_bindgen]
impl WithdrawApi for Contract {
    fn withdraw(&mut self, jar_index: JarIndex, amount: Option<U128>) -> PromiseOrValue<U128> {
        self.withdraw_internal(
            jar_index,
            amount.map(|value| value.0),
            Self::transfer_withdraw,
        )
    }
}

impl Contract {
    pub(crate) fn withdraw_internal(
        &mut self,
        jar_index: JarIndex,
        amount: Option<TokenAmount>,
        withdraw_transfer: WithdrawFunction,
    ) -> PromiseOrValue<U128> {
        let jar = self.get_jar_internal(jar_index).locked();

        assert_sufficient_balance(&jar, amount);
        assert_is_not_empty(&jar);
        assert_is_not_closed(&jar);

        let now = env::block_timestamp_ms();
        let product = self.get_product(&jar.product_id);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);
        assert_is_mature(&jar, &product, now);

        self.do_transfer(&account_id, &jar, amount, withdraw_transfer)
    }

    pub(crate) fn after_withdraw_internal(
        &mut self,
        jar_before_transfer: Jar,
        withdrawn_amount: TokenAmount,
        is_promise_success: bool,
    ) -> U128 {
        let result = if is_promise_success {
            let product = self.get_product(&jar_before_transfer.product_id);
            let now = env::block_timestamp_ms();
            let jar = jar_before_transfer.withdrawn(&product, withdrawn_amount, now);

            self.jars.replace(jar_before_transfer.index, jar.unlocked());

            withdrawn_amount
        } else {
            self.jars.replace(jar_before_transfer.index, jar_before_transfer.unlocked());

            0
        };

        U128(result)
    }

    fn do_transfer(
        &mut self,
        account_id: &AccountId,
        jar: &Jar,
        amount: Option<TokenAmount>,
        withdraw_transfer: WithdrawFunction,
    ) -> PromiseOrValue<U128> {
        emit(EventKind::Withdraw(WithdrawData { index: jar.index }));

        self.jars.replace(jar.index, jar.locked());

        let amount = amount.unwrap_or(jar.principal);

        withdraw_transfer(self, account_id, amount, jar)
    }

    fn transfer_withdraw(&mut self, account_id: &AccountId, amount: TokenAmount, jar: &Jar) -> PromiseOrValue<U128> {
        let product = self.get_product(&jar.product_id);
        let fee = product.withdrawal_fee.map(|fee| match fee {
            WithdrawalFee::Fix(amount) => amount,
            WithdrawalFee::Percent(percent) => percent.mul(jar.principal)
        }).map(|fee| Fee {
            amount: fee,
            beneficiary_id: self.fee_account_id.clone(),
        });

        self.ft_contract()
            .transfer(account_id, amount, fee)
            .then(after_withdraw_call(jar.clone(), amount))
            .into()
    }
}

#[near_bindgen]
impl WithdrawCallbacks for Contract {
    #[private]
    fn after_withdraw(
        &mut self,
        jar_before_transfer: Jar,
        withdrawn_amount: TokenAmount,
    ) -> U128 {
        self.after_withdraw_internal(jar_before_transfer, withdrawn_amount, is_promise_success())
    }
}

fn after_withdraw_call(jar_before_transfer: Jar, withdrawn_balance: TokenAmount) -> Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
        .after_withdraw(jar_before_transfer, withdrawn_balance)
}

#[cfg(test)]
mod tests {
    use near_sdk::{AccountId, PromiseOrValue};
    use near_sdk::json_types::{U128, U64};
    use near_sdk::test_utils::accounts;
    use crate::common::tests::Context;
    use crate::common::TokenAmount;
    use crate::Contract;
    use crate::jar::model::{Jar, JarTicket};
    use crate::product::api::ProductApi;
    use crate::product::model::Product;
    use crate::product::tests::get_register_product_command;

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_locked_jar_before_maturity_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.contract.withdraw_internal(0, None, transfer_withdraw);
    }

    #[test]
    #[should_panic(expected = "The jar is not mature yet")]
    fn withdraw_locked_jar_before_maturity_by_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        context.switch_account(&admin);
        context.contract.register_product(get_register_product_command());

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.switch_account(&alice);
        context.contract.withdraw_internal(0, None, transfer_withdraw);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_locked_jar_after_maturity_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_product_command();
        let product: Product = register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_ms(product.lockup_term + 1);

        context.contract.withdraw_internal(0, None, transfer_withdraw);
    }

    #[test]
    fn withdraw_locked_jar_after_maturity_by_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_product_command();
        let product: Product = register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_ms(product.lockup_term + 1);

        context.switch_account(&alice);
        context.contract.withdraw_internal(0, None, transfer_withdraw);
    }

    #[test]
    fn withdraw_flexible_jar_by_not_owner() {
        todo!()
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_with_sufficient_balance() {
        todo!()
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_with_insufficient_balance() {
        todo!()
    }

    fn transfer_withdraw(
        _: &mut Contract,
        _: &AccountId,
        amount: TokenAmount,
        _: &Jar,
    ) -> PromiseOrValue<U128> {
        PromiseOrValue::Value(U128(amount))
    }
}
