use near_sdk::{
    env, ext_contract,
    json_types::U128,
    near_bindgen, require,
    serde::{Deserialize, Serialize},
    AccountId, PromiseOrValue,
    PromiseOrValue::Value,
};
use sweat_jar_model::{api::RestakeApi, ProductId, TokenAmount};

use crate::{
    event::{emit, EventKind, EventKind::RestakeAll, RestakeAllData},
    internal::is_promise_success,
    Contract, ContractExt,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(super) struct Request {
    pub account_id: AccountId,
    pub withdrawal: WithdrawalDto,
    pub deposit: DepositDto,
    pub partitions: Vec<(ProductId, usize)>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub(super) struct WithdrawalDto {
    pub amount: TokenAmount,
    pub fee: TokenAmount,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub(super) struct DepositDto {
    pub product_id: ProductId,
    pub amount: TokenAmount,
}

#[near_bindgen]
impl RestakeApi for Contract {
    fn restake_all(&mut self, product_id: ProductId, amount: Option<U128>) -> PromiseOrValue<()> {
        self.get_product(&product_id).assert_enabled();

        let account_id = env::predecessor_account_id();

        self.assert_migrated(&account_id);

        let mut event_data = RestakeAllData::new(env::block_timestamp_ms(), vec![], product_id.clone(), 0, 0);

        let mut partition_indices: Vec<(ProductId, usize)> = vec![];
        let mut total_mature_balance = 0;
        let mut total_fee = 0;

        for (product_id, jar) in &self.get_account(&account_id).jars {
            if jar.is_pending_withdraw {
                continue;
            }

            let product = self.get_product(product_id);
            let (balance, partition_index) = jar.get_liquid_balance(&product.terms);

            if balance > 0 {
                total_mature_balance += balance;
                total_fee += product.calculate_fee(balance);
                partition_indices.push((product_id.clone(), partition_index));
                event_data.from.push(product_id.clone());
            }
        }

        for (product_id, _) in &partition_indices {
            self.update_jar_cache(&account_id, product_id);
        }

        let target_amount = amount.map_or(total_mature_balance, |amount| amount.0);
        require!(target_amount <= total_mature_balance, "Not enough funds to restake");

        let mut request = Request {
            account_id,
            withdrawal: WithdrawalDto::default(),
            deposit: DepositDto {
                product_id,
                amount: target_amount,
            },
            partitions: partition_indices,
        };

        let withdrawal_amount = total_mature_balance - target_amount;

        event_data.restaked = target_amount.into();
        event_data.withdrawn = withdrawal_amount.into();
        let event = RestakeAll(event_data);

        if withdrawal_amount > 0 {
            let withdrawal_fee = total_fee * withdrawal_amount / total_mature_balance;
            request.withdrawal = WithdrawalDto {
                amount: withdrawal_amount,
                fee: withdrawal_fee,
            };

            self.transfer_remainder(request, event)
        } else {
            self.clean_up_and_deposit(request);
            emit(event);

            Value(())
        }
    }
}

pub(super) trait RemainderTransfer {
    fn transfer_remainder(&mut self, request: Request, event: EventKind) -> PromiseOrValue<()>;
}

#[allow(dead_code)] // False positive since rust 1.78. It is used from `ext_contract` macro.
#[ext_contract(ext_self)]
pub(super) trait RemainderTransferCallback {
    fn after_transfer_remainder(&mut self, request: Request, event: EventKind) -> PromiseOrValue<()>;
}

#[near_bindgen]
impl RemainderTransferCallback for Contract {
    #[private]
    fn after_transfer_remainder(&mut self, request: Request, event: EventKind) -> PromiseOrValue<()> {
        for (product_id, _) in &request.partitions {
            self.get_account_mut(&request.account_id)
                .get_jar_mut(product_id)
                .unlock();
        }

        if is_promise_success() {
            self.clean_up_and_deposit(request);
            emit(event);
        }

        Value(())
    }
}

impl Contract {
    pub(super) fn clean_up_and_deposit(&mut self, request: Request) {
        let account = self.get_account_mut(&request.account_id);

        for (product_id, partition_index) in &request.partitions {
            account.get_jar_mut(product_id).clean_up_deposits(*partition_index);
        }

        self.get_account_mut(&request.account_id)
            .deposit(&request.deposit.product_id, request.deposit.amount, None);
    }
}
