use near_sdk::{ext_contract, is_promise_success, json_types::U128, near_bindgen, PromiseOrValue};

use crate::{
    assert::{assert_is_liquidable, assert_sufficient_balance},
    assert_is_not_closed, assert_ownership,
    common::{TokenAmount, GAS_FOR_AFTER_TRANSFER},
    env,
    event::{emit, EventKind, WithdrawData},
    ft_interface::Fee,
    jar::view::JarIndexView,
    product::model::WithdrawalFee,
    withdraw::view::WithdrawView,
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
            let jar = jar_before_transfer.withdrawn(product, withdrawn_amount, now);

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
            .with_static_gas(GAS_FOR_AFTER_TRANSFER)
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
    use near_sdk::{json_types::U128, test_utils::accounts, AccountId, PromiseOrValue};

    use crate::{
        common::{tests::Context, u32::U32, udecimal::UDecimal},
        jar::{api::JarApi, model::Jar},
        product::model::{Product, WithdrawalFee},
        withdraw::api::WithdrawApi,
    };

    fn prepare_jar(product: &Product) -> (AccountId, Jar, Context) {
        let alice = accounts(0);
        let admin = accounts(1);

        let jar = Jar::generate(0, &alice, &product.id).principal(1_000_000);
        let context = Context::new(admin)
            .with_products(&[product.clone()])
            .with_jars(&[jar.clone()]);

        (alice, jar, context)
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_locked_jar_before_maturity_by_not_owner() {
        let (_, _, mut context) = prepare_jar(&generate_product());

        context.contract.withdraw(U32(0), None);
    }

    #[test]
    #[should_panic(expected = "The jar is not mature yet")]
    fn withdraw_locked_jar_before_maturity_by_owner() {
        let (alice, jar, mut context) = prepare_jar(&generate_product());

        context.switch_account(&alice);
        context.contract.withdraw(U32(jar.index), None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_locked_jar_after_maturity_by_not_owner() {
        let product = generate_product();
        let (_, jar, mut context) = prepare_jar(&product);

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
        context.contract.withdraw(U32(jar.index), None);
    }

    #[test]
    fn withdraw_locked_jar_after_maturity_by_owner() {
        let product = generate_product();
        let (alice, jar, mut context) = prepare_jar(&product);

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
        context.switch_account(&alice);
        context.contract.withdraw(U32(jar.index), None);
    }

    #[test]
    #[should_panic(expected = "Account doesn't own this jar")]
    fn withdraw_flexible_jar_by_not_owner() {
        let product = generate_flexible_product();
        let (_, jar, mut context) = prepare_jar(&product);

        context.set_block_timestamp_in_days(1);
        context.contract.withdraw(U32(jar.index), None);
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_full() {
        let product = generate_flexible_product();
        let (alice, reference_jar, mut context) = prepare_jar(&product);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(U32(reference_jar.index), None);
        let jar = context.contract.get_jar(U32(reference_jar.index));
        assert_eq!(0, jar.principal.0);
    }

    #[test]
    fn withdraw_flexible_jar_by_owner_with_sufficient_balance() {
        let product = generate_flexible_product();
        let (alice, reference_jar, mut context) = prepare_jar(&product);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);

        context.contract.withdraw(U32(0), Some(U128(100_000)));
        let jar = context.contract.get_jar(U32(reference_jar.index));
        assert_eq!(900_000, jar.principal.0);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn withdraw_flexible_jar_by_owner_with_insufficient_balance() {
        let product = generate_flexible_product();
        let (alice, jar, mut context) = prepare_jar(&product);

        context.set_block_timestamp_in_days(1);
        context.switch_account(&alice);
        context.contract.withdraw(U32(jar.index), Some(U128(2_000_000)));
    }

    #[test]
    fn product_with_fixed_fee() {
        let fee = 10;
        let product = generate_product_with_fee(&WithdrawalFee::Fix(fee));
        let (alice, reference_jar, mut context) = prepare_jar(&product);

        let initial_principal = reference_jar.principal;

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
        context.switch_account(&alice);

        let withdraw_amount = 100_000;
        let PromiseOrValue::Value(withdraw) = context.contract.withdraw(U32(0), Some(U128(withdraw_amount))) else {
            panic!("Invalid promise type");
        };

        assert_eq!(withdraw.withdrawn_amount, U128(withdraw_amount - fee));
        assert_eq!(withdraw.fee, U128(fee));

        let jar = context.contract.get_jar(U32(reference_jar.index));

        assert_eq!(jar.principal, U128(initial_principal - withdraw_amount));
    }

    #[test]
    fn product_with_percent_fee() {
        let fee_value = UDecimal::new(5, 4);
        let fee = WithdrawalFee::Percent(fee_value.clone());
        let product = generate_product_with_fee(&fee);
        let (alice, reference_jar, mut context) = prepare_jar(&product);

        let initial_principal = reference_jar.principal;

        context.set_block_timestamp_in_ms(product.get_lockup_term().unwrap() + 1);
        context.switch_account(&alice);

        let withdrawn_amount = 100_000;
        let PromiseOrValue::Value(withdraw) = context.contract.withdraw(U32(0), Some(U128(withdrawn_amount))) else {
            panic!("Invalid promise type");
        };

        let reference_fee = fee_value * initial_principal;
        assert_eq!(withdraw.withdrawn_amount, U128(withdrawn_amount - reference_fee));
        assert_eq!(withdraw.fee, U128(reference_fee));

        let jar = context.contract.get_jar(U32(reference_jar.index));

        assert_eq!(jar.principal, U128(initial_principal - withdrawn_amount));
    }

    #[test]
    fn test_failed_withdraw() {
        let product = generate_product();
        let (_, reference_jar, mut context) = prepare_jar(&product);

        let jar_view = context.contract.get_jar(U32(reference_jar.index));
        let jar = context.contract.jars[0].clone();
        let withdraw = context.contract.after_withdraw_internal(jar, 1234, None, false);

        assert_eq!(withdraw.withdrawn_amount, U128(0));
        assert_eq!(withdraw.fee, U128(0));

        assert_eq!(jar_view, context.contract.get_jar(U32(0)));
    }

    pub(crate) fn generate_product() -> Product {
        Product::generate("product").enabled(true)
    }

    pub(crate) fn generate_flexible_product() -> Product {
        Product::generate("flexible_product").enabled(true).flexible()
    }

    pub(crate) fn generate_product_with_fee(fee: &WithdrawalFee) -> Product {
        Product::generate("product_with_fee")
            .enabled(true)
            .with_withdrawal_fee(fee.clone())
    }
}
