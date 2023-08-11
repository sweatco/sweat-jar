use near_sdk::{ext_contract, is_promise_success, near_bindgen, PromiseOrValue};
use near_sdk::json_types::U128;

use crate::*;
use crate::assert::{assert_is_liquidable, assert_sufficient_balance};
use crate::common::TokenAmount;
use crate::event::{emit, EventKind, WithdrawData};
use crate::external::GAS_FOR_AFTER_TRANSFER;
use crate::ft_interface::Fee;
use crate::ft_interface::FungibleTokenInterface;
use crate::jar::model::JarIndex;
use crate::product::model::WithdrawalFee;

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
        let jar = self.get_jar_internal(jar_index).locked();
        let amount = amount.map_or_else(
            || jar.principal,
            |value| value.0,
        );

        assert_sufficient_balance(&jar, amount);
        assert_is_not_empty(&jar);
        assert_is_not_closed(&jar);

        let now = env::block_timestamp_ms();
        let product = self.get_product(&jar.product_id);
        let account_id = env::predecessor_account_id();

        assert_ownership(&jar, &account_id);
        assert_is_liquidable(&jar, &product, now);

        self.do_transfer(&account_id, &jar, amount)
    }
}

impl Contract {
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
        amount: TokenAmount,
    ) -> PromiseOrValue<U128> {
        emit(EventKind::Withdraw(WithdrawData { index: jar.index }));

        self.jars.replace(jar.index, jar.locked());

        self.transfer_withdraw(account_id, amount, jar)
    }
}

#[cfg(not(test))]
impl Contract {
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
            .then(Self::after_withdraw_call(jar.clone(), amount))
            .into()
    }

    fn after_withdraw_call(jar_before_transfer: Jar, withdrawn_balance: TokenAmount) -> Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
            .after_withdraw(jar_before_transfer, withdrawn_balance)
    }
}

#[cfg(test)]
impl Contract {
    fn transfer_withdraw(&mut self, _: &AccountId, amount: TokenAmount, jar: &Jar) -> PromiseOrValue<U128> {
        self.after_withdraw_internal(jar.clone(), amount, true);

        PromiseOrValue::Value(U128(amount))
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

#[cfg(test)]
mod tests {
    use near_sdk::json_types::{U128, U64};
    use near_sdk::test_utils::accounts;

    use crate::common::tests::Context;
    use crate::jar::api::JarApi;
    use crate::jar::model::JarTicket;
    use crate::product::api::ProductApi;
    use crate::product::model::Product;
    use crate::product::tests::{get_register_flexible_product_command, get_register_product_command};
    use crate::withdraw::api::WithdrawApi;

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
        context.contract.create_jar(alice, ticket, U128(1_000_000), None);

        context.contract.withdraw(0, None);
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
        context.contract.withdraw(0, None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_locked_jar_after_maturity_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_product_command();
        let product: &Product = &register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: product.id.clone(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);

        context.contract.withdraw(0, None);
    }

    #[test]
    fn withdraw_locked_jar_after_maturity_by_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_product_command();
        let product: &Product = &register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: product.clone().id,
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);

        context.switch_account(&alice);
        context.contract.withdraw(0, None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_flexible_jar_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_flexible_product_command();
        let product: Product = register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.contract.withdraw(0, None);
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_full() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_flexible_product_command();
        let product: Product = register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(0, None);
        let jar = context.contract.get_jar(0);
        assert_eq!(0, jar.principal.0);
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_with_sufficient_balance() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_flexible_product_command();
        let product: Product = register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(0, Some(U128(100_000)));
        let jar = context.contract.get_jar(0);
        assert_eq!(900_000, jar.principal.0);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn withdraw_flexible_jar_by_owner_with_insufficient_balance() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(vec![admin.clone()]);

        let register_product_command = get_register_flexible_product_command();
        let product: Product = register_product_command.clone().into();
        context.switch_account(&admin);
        context.contract.register_product(register_product_command);

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context.contract.create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(0, Some(U128(2_000_000)));
    }
}
