use near_sdk::{ext_contract, is_promise_success, json_types::U128, near_bindgen, PromiseOrValue};

use crate::{
    assert::{assert_is_liquidable, assert_sufficient_balance},
    assert_is_not_closed, assert_ownership,
    common::TokenAmount,
    env,
    event::{emit, EventKind, WithdrawData},
    ft_interface::Fee,
    jar::view::JarIndexView,
    product::model::WithdrawalFee,
    withdraw::view::WithdrawView,
    AccountId, Contract, ContractExt, Jar, Product,
};
#[cfg(not(test))]
use crate::{
    ft_interface::{FungibleTokenInterface, GAS_FOR_AFTER_TRANSFER},
    Gas, Promise,
};

/// The `WithdrawApi` trait defines methods for withdrawing tokens from specific deposit jars within the smart contract.
pub trait WithdrawApi {
    /// Allows the owner of a deposit jar to withdraw a specified amount of tokens from it.
    ///
    /// # Arguments
    ///
    /// * `jar_index` - The index of the deposit jar from which the withdrawal is being made.
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
    fn withdraw(&mut self, jar_index: JarIndexView, amount: Option<U128>) -> PromiseOrValue<WithdrawView>;
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
    fn withdraw(&mut self, jar_index: JarIndexView, amount: Option<U128>) -> PromiseOrValue<WithdrawView> {
        let jar = self.get_jar_internal(jar_index.0).locked();
        let amount = amount.map_or(jar.principal, |value| value.0);

        let account_id = env::predecessor_account_id();
        assert_ownership(&jar, &account_id);

        assert_sufficient_balance(&jar, amount);
        assert_is_not_closed(&jar);

        let now = env::block_timestamp_ms();
        let product = self.get_product(&jar.product_id);

        assert_is_liquidable(&jar, &product, now);

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
            let jar = jar_before_transfer.withdrawn(&product, withdrawn_amount, now);

            self.jars.replace(jar_before_transfer.index, jar.unlocked());

            emit(EventKind::Withdraw(WithdrawData { index: jar.index }));

            WithdrawView::new(withdrawn_amount, fee)
        } else {
            self.jars
                .replace(jar_before_transfer.index, jar_before_transfer.unlocked());

            WithdrawView::new(0, None)
        }
    }

    fn do_transfer(&mut self, account_id: &AccountId, jar: &Jar, amount: TokenAmount) -> PromiseOrValue<WithdrawView> {
        self.jars.replace(jar.index, jar.locked());

        self.transfer_withdraw(account_id, amount, jar)
    }

    fn get_fee(&self, product: &Product, jar: &Jar) -> Option<Fee> {
        product
            .withdrawal_fee
            .clone()
            .map(|fee| match fee {
                WithdrawalFee::Fix(amount) => amount,
                WithdrawalFee::Percent(percent) => percent.mul(jar.principal),
            })
            .map(|fee| Fee {
                amount: fee,
                beneficiary_id: self.fee_account_id.clone(),
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
        let fee = self.get_fee(&product, jar);

        self.ft_contract()
            .transfer(account_id, amount, "withdraw", &fee)
            .then(Self::after_withdraw_call(jar.clone(), amount, &fee))
            .into()
    }

    fn after_withdraw_call(jar_before_transfer: Jar, withdrawn_balance: TokenAmount, fee: &Option<Fee>) -> Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
            .after_withdraw(jar_before_transfer, withdrawn_balance, fee.clone())
    }
}

#[cfg(test)]
impl Contract {
    fn transfer_withdraw(&mut self, _: &AccountId, amount: TokenAmount, jar: &Jar) -> PromiseOrValue<WithdrawView> {
        let product = self.get_product(&jar.product_id);
        let fee = self.get_fee(&product, jar);

        self.after_withdraw_internal(jar.clone(), amount, fee.clone(), true);

        PromiseOrValue::Value(WithdrawView::new(amount, fee))
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

#[cfg(test)]
mod tests {
    use near_sdk::{
        json_types::{U128, U64},
        test_utils::accounts,
    };

    use crate::{
        common::{tests::Context, U32},
        jar::{api::JarApi, model::JarTicket},
        product::{
            api::ProductApi,
            command::WithdrawalFeeDto,
            model::Product,
            tests::{get_fee_product_command, get_register_flexible_product_command, get_register_product_command},
        },
        withdraw::api::WithdrawApi,
    };

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_locked_jar_before_maturity_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let ticket = JarTicket {
            product_id: "product".to_string(),
            valid_until: U64(0),
        };
        context.contract.create_jar(alice, ticket, U128(1_000_000), None);

        context.contract.withdraw(U32(0), None);
    }

    #[test]
    #[should_panic(expected = "The jar is not mature yet")]
    fn withdraw_locked_jar_before_maturity_by_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let ticket = JarTicket {
            product_id: "product".to_string(),

            valid_until: U64(0),
        };

        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.switch_account(&alice);
        context.contract.withdraw(U32(0), None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_locked_jar_after_maturity_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        let product: &Product = &get_register_product_command().into();
        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let ticket = JarTicket {
            product_id: product.id.clone(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);

        context.contract.withdraw(U32(0), None);
    }

    #[test]
    fn withdraw_locked_jar_after_maturity_by_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        let product: Product = get_register_product_command().into();
        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context.contract.register_product(get_register_product_command())
        });

        let ticket = JarTicket {
            product_id: product.id.clone(),
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);

        context.switch_account(&alice);
        context.contract.withdraw(U32(0), None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_flexible_jar_by_not_owner() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        let product: Product = get_register_flexible_product_command().into();
        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_flexible_product_command())
        });

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.contract.withdraw(U32(0), None);
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_full() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        let product: Product = get_register_flexible_product_command().into();
        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_flexible_product_command())
        });

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(U32(0), None);
        let jar = context.contract.get_jar(U32(0));
        assert_eq!(0, jar.principal.0);
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_with_sufficient_balance() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        let product: Product = get_register_flexible_product_command().into();
        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_flexible_product_command())
        });

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(U32(0), Some(U128(100_000)));
        let jar = context.contract.get_jar(U32(0));
        assert_eq!(900_000, jar.principal.0);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn withdraw_flexible_jar_by_owner_with_insufficient_balance() {
        let alice = accounts(0);
        let admin = accounts(1);
        let mut context = Context::new(admin.clone());

        let product: Product = get_register_flexible_product_command().into();
        context.switch_account(&admin);
        context.with_deposit_yocto(1, |context| {
            context
                .contract
                .register_product(get_register_flexible_product_command())
        });

        let ticket = JarTicket {
            product_id: product.id,
            valid_until: U64(0),
        };
        context
            .contract
            .create_jar(alice.clone(), ticket, U128(1_000_000), None);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(U32(0), Some(U128(2_000_000)));
    }

    #[test]
    fn product_with_fee() {
        // let alice = accounts(0);
        // let admin = accounts(1);
        // let mut context = Context::new(admin.clone());
        //
        // let product: Product = get_fee_product_command(WithdrawalFeeDto::Fix(U128(10))).into();
        // context.switch_account(&admin);
        // context.with_deposit_yocto(1, |context| {
        //     context
        //         .contract
        //         .register_product(get_register_flexible_product_command())
        // });
        //
        // let ticket = JarTicket {
        //     product_id: product.id,
        //     valid_until: U64(0),
        // };
        //
        // context
        //     .contract
        //     .create_jar(alice.clone(), ticket, U128(1_000_000), None);
    }
}
