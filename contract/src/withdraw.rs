use near_sdk::{Balance, ext_contract, is_promise_success, near_bindgen, PromiseOrValue};
use crate::*;
use crate::assert::assert_is_mature;
use crate::external::GAS_FOR_AFTER_TRANSFER;
use crate::jar::JarIndex;

pub(crate) type WithdrawFunction = fn(
    contract: &mut Contract,
    account_id: &AccountId,
    amount: Balance,
    jar: &Jar,
) -> PromiseOrValue<Balance>;

pub trait WithdrawApi {
    fn withdraw(&mut self, jar_id: JarIndex, amount: Option<Balance>) -> PromiseOrValue<Balance>;
}

#[ext_contract(ext_self)]
pub trait WithdrawCallbacks {
    fn after_withdraw(&mut self, jar_before_transfer: Jar, withdrawn_amount: Balance);
}

#[near_bindgen]
impl WithdrawApi for Contract {
    fn withdraw(&mut self, jar_index: JarIndex, amount: Option<Balance>) -> PromiseOrValue<Balance> {
        self.withdraw_internal(jar_index, amount, Self::transfer_withdraw)
    }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub(crate) fn withdraw_internal(
        &mut self,
        jar_index: JarIndex,
        amount: Option<Balance>,
        withdraw_transfer: WithdrawFunction,
    ) -> PromiseOrValue<Balance> {
        let jar = self.get_jar(jar_index).locked();
        assert_is_not_empty(&jar);
        assert_is_not_closed(&jar);

        let now = env::block_timestamp_ms();
        let product = self.get_product(&jar.product_id);
        let account_id = env::predecessor_account_id();

        if let Some(notice_term) = product.notice_term {
            if let JarState::Noticed(noticed_at) = jar.state {
                if now - noticed_at >= notice_term {
                    return self.do_transfer(&account_id, &jar, amount, withdraw_transfer);
                }
            } else {
                assert_ownership(&jar, &account_id);
                assert_is_mature(&jar, &product, now);

                return self.do_notice(&jar);
            }
        } else {
            assert_ownership(&jar, &account_id);
            assert_is_mature(&jar, &product, now);

            return self.do_transfer(&account_id, &jar, amount, withdraw_transfer);
        }

        PromiseOrValue::Value(0)
    }

    #[private]
    fn do_transfer(
        &mut self,
        account_id: &AccountId,
        jar: &Jar,
        amount: Option<Balance>,
        withdraw_transfer: WithdrawFunction,
    ) -> PromiseOrValue<Balance> {
        let event = json!({
                "standard": "sweat_jar",
                "version": "0.0.1",
                "event": "withdraw",
                "data": {
                    "index": jar.index,
                    "action": "withdrawn",
                },
            });
        env::log_str(format!("EVENT_JSON: {}", event.to_string().as_str()).as_str());

        self.jars.replace(jar.index, &jar.locked());

        let amount = amount.unwrap_or(jar.principal);

        withdraw_transfer(self, &account_id, amount, jar)
    }

    #[private]
    fn do_notice(&mut self, jar: &Jar) -> PromiseOrValue<Balance> {
        let event = json!({
                    "standard": "sweat_jar",
                    "version": "0.0.1",
                    "event": "withdraw",
                    "data": {
                        "index": jar.index,
                        "action": "noticed",
                    },
                });
        env::log_str(format!("EVENT_JSON: {}", event.to_string().as_str()).as_str());

        let noticed_jar = jar.noticed(env::block_timestamp_ms());
        self.jars.replace(noticed_jar.index, &noticed_jar);

        PromiseOrValue::Value(0)
    }
}

#[near_bindgen]
impl WithdrawCallbacks for Contract {
    fn after_withdraw(
        &mut self,
        jar_before_transfer: Jar,
        withdrawn_amount: Balance,
    ) {
        self.after_withdraw_internal(jar_before_transfer, withdrawn_amount, is_promise_success())
    }
}

#[near_bindgen]
impl Contract {
    fn transfer_withdraw(&mut self, account_id: &AccountId, amount: Balance, jar: &Jar) -> PromiseOrValue<Balance> {
        self.ft_contract().transfer(
            account_id.clone(),
            amount,
            after_withdraw_call(jar.clone(), amount),
        )
    }

    pub(crate) fn after_withdraw_internal(
        &mut self,
        jar_before_transfer: Jar,
        withdrawn_amount: Balance,
        is_promise_success: bool,
    ) {
        println!("@@ after_withdraw");

        if is_promise_success {
            let product = self.get_product(&jar_before_transfer.product_id);
            let now = env::block_timestamp_ms();
            let jar = jar_before_transfer.withdrawn(&product, withdrawn_amount, now);

            self.jars.replace(jar_before_transfer.index, &jar.unlocked());
        } else {
            self.jars.replace(jar_before_transfer.index, &jar_before_transfer.unlocked());
        }
    }
}

fn after_withdraw_call(jar_before_transfer: Jar, withdrawn_balance: Balance) -> Promise {
    println!("@@ after_withdraw_call");
    ext_self::ext(env::current_account_id())
        .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
        .after_withdraw(jar_before_transfer, withdrawn_balance)
}
