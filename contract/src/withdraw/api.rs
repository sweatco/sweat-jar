use near_sdk::{
    ext_contract, is_promise_success,
    json_types::U128,
    near_bindgen,
    serde::{Deserialize, Serialize},
    PromiseOrValue,
};
use sweat_jar_model::{
    api::WithdrawApi,
    jar::{JarId, JarIdView},
    withdraw::{BulkWithdrawView, Fee, WithdrawView},
    TokenAmount,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct JarWithdraw {
    pub jar: Jar,
    pub should_be_closed: bool,
    pub amount: u128,
    pub fee: Option<TokenAmount>,
}

#[cfg(not(test))]
use crate::ft_interface::FungibleTokenInterface;
use crate::{
    assert::{assert_is_liquidable, assert_not_locked, assert_sufficient_balance},
    env,
    event::{emit, EventKind, WithdrawData},
    jar::model::Jar,
    product::model::WithdrawalFee,
    AccountId, Contract, ContractExt, Product,
};

impl Contract {
    fn can_be_withdrawn(jar: &Jar, product: &Product, now: u64) -> bool {
        !jar.is_pending_withdraw && jar.is_liquidable(product, now)
    }
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

    fn after_bulk_withdraw(&mut self, account_id: AccountId, jars: Vec<JarWithdraw>) -> BulkWithdrawView;
}

#[near_bindgen]
impl WithdrawApi for Contract {
    fn withdraw(&mut self, jar_id: JarIdView, amount: Option<U128>) -> PromiseOrValue<WithdrawView> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_jars_if_needed(account_id.clone());

        let jar = self.get_jar_internal(&account_id, jar_id.0).clone();

        assert_not_locked(&jar);

        let amount = amount.map_or(jar.principal, |value| value.0);

        assert_sufficient_balance(&jar, amount);

        let now = env::block_timestamp_ms();
        let product = self.get_product(&jar.product_id);

        assert_is_liquidable(&jar, &product, now);

        let mut withdrawn_jar = jar.withdrawn(&product, amount, now);
        let close_jar = withdrawn_jar.should_be_closed(&product, now);

        withdrawn_jar.lock();
        *self.get_jar_mut_internal(&jar.account_id, jar.id) = withdrawn_jar;

        self.transfer_withdraw(&account_id, amount, &jar, close_jar)
    }

    fn withdraw_all(&mut self) -> PromiseOrValue<BulkWithdrawView> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_jars_if_needed(account_id.clone());
        let now = env::block_timestamp_ms();

        let Some(account_jars) = self.account_jars.get(&account_id) else {
            return PromiseOrValue::Value(BulkWithdrawView::default());
        };

        let jars: Vec<JarWithdraw> = account_jars
            .jars
            .clone()
            .into_iter()
            .filter_map(|jar| {
                let product = self.get_product(&jar.product_id);

                if !Self::can_be_withdrawn(&jar, &product, now) {
                    return None;
                }

                let amount = jar.principal;

                if amount == 0 {
                    return None;
                }

                let mut withdrawn_jar = jar.withdrawn(&product, amount, now);
                let should_be_closed = withdrawn_jar.should_be_closed(&product, now);

                withdrawn_jar.lock();
                *self.get_jar_mut_internal(&jar.account_id, jar.id) = withdrawn_jar;

                JarWithdraw {
                    jar,
                    should_be_closed,
                    amount,
                    fee: None,
                }
                .into()
            })
            .collect();

        if jars.is_empty() {
            return PromiseOrValue::Value(BulkWithdrawView::default());
        }

        self.transfer_bulk_withdraw(&account_id, jars)
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
            amount: withdrawal_result.withdrawn_amount,
            fee: withdrawal_result.fee,
        }));

        withdrawal_result
    }

    pub(crate) fn after_bulk_withdraw_internal(
        &mut self,
        account_id: AccountId,
        jars: Vec<JarWithdraw>,
        is_promise_success: bool,
    ) -> BulkWithdrawView {
        let mut withdrawal_result = BulkWithdrawView {
            total_amount: 0.into(),
            jars: vec![],
        };

        if !is_promise_success {
            for withdraw in jars {
                let jar = self.get_jar_mut_internal(&account_id, withdraw.jar.id);
                jar.principal += withdraw.amount;
                jar.unlock();
            }
            return withdrawal_result;
        }

        let mut event_data = vec![];

        for withdraw in jars {
            if withdraw.should_be_closed {
                self.delete_jar(&account_id, withdraw.jar.id);
            } else {
                self.get_jar_mut_internal(&account_id, withdraw.jar.id).unlock();
            }

            let jar_result = WithdrawView::new(withdraw.amount, self.make_fee(withdraw.fee));

            event_data.push(WithdrawData {
                id: withdraw.jar.id,
                amount: jar_result.withdrawn_amount,
                fee: jar_result.fee,
            });

            withdrawal_result.total_amount.0 += jar_result.withdrawn_amount.0;
            withdrawal_result.jars.push(jar_result);
        }

        emit(EventKind::WithdrawAll(event_data));

        withdrawal_result
    }

    fn get_fee(product: &Product, jar: &Jar) -> Option<TokenAmount> {
        let fee = product.withdrawal_fee.as_ref()?;

        let amount = match fee {
            WithdrawalFee::Fix(amount) => *amount,
            WithdrawalFee::Percent(percent) => percent * jar.principal,
        };

        amount.into()
    }

    fn make_fee(&self, amount: Option<TokenAmount>) -> Option<Fee> {
        Fee {
            beneficiary_id: self.fee_account_id.clone(),
            amount: amount?,
        }
        .into()
    }
}

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
impl Contract {
    fn transfer_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: TokenAmount,
        jar: &Jar,
        close_jar: bool,
    ) -> PromiseOrValue<WithdrawView> {
        let product = self.get_product(&jar.product_id);
        let fee = Self::get_fee(&product, jar);

        self.ft_contract()
            .ft_transfer(account_id, amount, "withdraw", &self.make_fee(fee))
            .then(Self::after_withdraw_call(
                account_id.clone(),
                jar.id,
                close_jar,
                amount,
                &self.make_fee(fee),
            ))
            .into()
    }

    fn transfer_bulk_withdraw(
        &mut self,
        account_id: &AccountId,
        jars: Vec<JarWithdraw>,
    ) -> PromiseOrValue<BulkWithdrawView> {
        let total_fee: TokenAmount = jars
            .iter()
            .filter_map(|j| {
                let product = self.get_product(&j.jar.product_id);
                Self::get_fee(&product, &j.jar)
            })
            .sum();

        let total_fee = match total_fee {
            0 => None,
            _ => self.make_fee(total_fee.into()),
        };

        let total_amount = jars.iter().map(|j| j.amount).sum();

        let gas_left = env::prepaid_gas().as_gas() - env::used_gas().as_gas();
        let gas_needed = crate::common::gas_data::GAS_FOR_FT_TRANSFER.as_gas()
            + crate::common::gas_data::GAS_FOR_BULK_AFTER_WITHDRAW.as_gas();

        if gas_left < gas_needed {
            env::panic_str(&format!(
                r#"
                    Not enough gas left to complete transfer_bulk_withdraw. Number of jars: {}.
                    Left: {gas_left} Needed: {gas_needed}.
                    Consider attaching more gas to the transaction.
                "#,
                jars.len()
            ));
        }

        self.ft_contract()
            .ft_transfer(account_id, total_amount, "bulk_withdraw", &total_fee)
            .then(Self::after_bulk_withdraw_call(account_id.clone(), jars))
            .into()
    }

    fn after_withdraw_call(
        account_id: AccountId,
        jar_id: JarId,
        close_jar: bool,
        withdrawn_balance: TokenAmount,
        fee: &Option<Fee>,
    ) -> near_sdk::Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_WITHDRAW)
            .after_withdraw(account_id, jar_id, close_jar, withdrawn_balance, fee.clone())
    }

    fn after_bulk_withdraw_call(account_id: AccountId, jars: Vec<JarWithdraw>) -> near_sdk::Promise {
        ext_self::ext(env::current_account_id())
            .with_static_gas(crate::common::gas_data::GAS_FOR_BULK_AFTER_WITHDRAW)
            .after_bulk_withdraw(account_id, jars)
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
        let fee = Self::get_fee(&product, jar);

        let withdrawn = self.after_withdraw_internal(
            account_id.clone(),
            jar.id,
            close_jar,
            amount,
            self.make_fee(fee),
            crate::common::test_data::get_test_future_success(),
        );

        PromiseOrValue::Value(withdrawn)
    }

    fn transfer_bulk_withdraw(
        &mut self,
        account_id: &AccountId,
        jars: Vec<JarWithdraw>,
    ) -> PromiseOrValue<BulkWithdrawView> {
        let withdrawn = self.after_bulk_withdraw_internal(
            account_id.clone(),
            jars,
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

    #[private]
    fn after_bulk_withdraw(&mut self, account_id: AccountId, jars: Vec<JarWithdraw>) -> BulkWithdrawView {
        self.after_bulk_withdraw_internal(account_id, jars, is_promise_success())
    }
}
