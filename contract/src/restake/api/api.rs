use near_sdk::{
    env, ext_contract,
    json_types::{Base64VecU8, U128},
    near, require, AccountId, PromiseOrValue,
    PromiseOrValue::Value,
};
use sweat_jar_model::{api::RestakeApi, jar::DepositTicket, ProductId, TokenAmount};

use crate::{
    assert::assert_not_locked,
    event::{emit, EventKind, EventKind::RestakeAll, RestakeAllData},
    internal::is_promise_success,
    product::model::v1::{ProductAssertions, ProductModelApi},
    withdraw::api::WithdrawalDto,
    Contract, ContractExt,
};

#[derive(Debug)]
#[near(serializers=[json])]
pub(super) struct Request {
    pub account_id: AccountId,
    pub withdrawal: Option<WithdrawalDto>,
    pub deposit: DepositDto,
    pub partitions: Vec<(ProductId, usize)>,
}

#[derive(Debug)]
#[near(serializers=[json])]
pub(super) struct DepositDto {
    pub product_id: ProductId,
    pub amount: TokenAmount,
}

#[near]
impl RestakeApi for Contract {
    fn restake(
        &mut self,
        from: ProductId,
        ticket: DepositTicket,
        signature: Option<Base64VecU8>,
        amount: Option<U128>,
    ) -> PromiseOrValue<()> {
        let builder = RestakeRequestBuilder {
            account_id: env::predecessor_account_id(),
            from,
            ticket: ticket.clone(),
            target_amount: amount.map(|amount| amount.0),
        };

        self.restake_internal(&ticket, &signature, builder)
    }

    fn restake_all(
        &mut self,
        ticket: DepositTicket,
        signature: Option<Base64VecU8>,
        amount: Option<U128>,
    ) -> PromiseOrValue<()> {
        let builder = RestakeAllRequestBuilder {
            account_id: env::predecessor_account_id(),
            ticket: ticket.clone(),
            target_amount: amount.map(|amount| amount.0),
        };

        self.restake_internal(&ticket, &signature, builder)
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

#[near]
impl RemainderTransferCallback for Contract {
    #[private]
    fn after_transfer_remainder(&mut self, request: Request, event: EventKind) -> PromiseOrValue<()> {
        for (product_id, _) in &request.partitions {
            self.get_account_mut(&request.account_id)
                .get_jar_mut(product_id)
                .unlock();
        }

        if is_promise_success() {
            self.fee_amount += request.withdrawal.map_or(0, |w| w.fee);
            self.clean_up_and_deposit(request);
            emit(event);
        }

        Value(())
    }
}

impl Contract {
    fn restake_internal(
        &mut self,
        ticket: &DepositTicket,
        signature: &Option<Base64VecU8>,
        builder: impl RequestBuilder,
    ) -> PromiseOrValue<()> {
        let request = self.prepare_request_safely(ticket, signature, builder);
        let event = RestakeAll(request.account_id.clone(), RestakeAllData::from(&request));

        for (product_id, _) in request.partitions.iter() {
            self.update_jar_cache(&request.account_id, product_id);
        }

        if request.withdrawal.is_none() {
            self.clean_up_and_deposit(request);
            emit(event);

            return Value(());
        }

        for (product_id, _) in &request.partitions {
            self.get_account_mut(&request.account_id).get_jar_mut(product_id).lock();
        }

        self.transfer_remainder(request, event)
    }

    fn prepare_request_safely(
        &self,
        ticket: &DepositTicket,
        signature: &Option<Base64VecU8>,
        builder: impl RequestBuilder,
    ) -> Request {
        let product_id = ticket.product_id.clone();
        self.get_product(&product_id).assert_enabled();

        let request = builder.build(self);

        if request.deposit.amount == 0 {
            env::panic_str("Nothing to restake");
        }
        self.verify(&request.account_id, request.deposit.amount, &ticket, &signature);

        request
    }

    fn clean_up_and_deposit(&mut self, request: Request) {
        let account = self.get_account_mut(&request.account_id);

        for (product_id, partition_index) in &request.partitions {
            account.get_jar_mut(product_id).clean_up_deposits(*partition_index);
        }

        self.get_account_mut(&request.account_id)
            .deposit(&request.deposit.product_id, request.deposit.amount, None);

        if self.get_product(&request.deposit.product_id).is_protected() {
            self.get_account_mut(&request.account_id).nonce += 1;
        }
    }
}

trait RequestBuilder {
    fn build(&self, contract: &Contract) -> Request;
}

struct RestakeRequestBuilder {
    account_id: AccountId,
    from: ProductId,
    ticket: DepositTicket,
    target_amount: Option<TokenAmount>,
}

impl RequestBuilder for RestakeRequestBuilder {
    fn build(&self, contract: &Contract) -> Request {
        let jar = contract.get_account(&self.account_id).get_jar(&self.from);
        assert_not_locked(jar);

        let product = contract.get_product(&self.from);
        let (mature_balance, partition_index) = jar.get_liquid_balance(&product.terms);

        let deposit = DepositDto::new(self.ticket.product_id.clone(), mature_balance, self.target_amount);

        let withdrawal_amount = mature_balance - deposit.amount;
        let withdrawal = if withdrawal_amount > 0 {
            Some(WithdrawalDto {
                amount: withdrawal_amount,
                fee: product.calculate_fee(withdrawal_amount),
            })
        } else {
            None
        };

        Request {
            account_id: self.account_id.clone(),
            withdrawal,
            deposit,
            partitions: vec![(self.from.clone(), partition_index)],
        }
    }
}

struct RestakeAllRequestBuilder {
    account_id: AccountId,
    ticket: DepositTicket,
    target_amount: Option<TokenAmount>,
}

impl RequestBuilder for RestakeAllRequestBuilder {
    fn build(&self, contract: &Contract) -> Request {
        let mut partition_indices: Vec<(ProductId, usize)> = vec![];
        let mut total_mature_balance = 0;
        let mut total_fee = 0;

        for (product_id, jar) in &contract.get_account(&self.account_id).jars {
            if jar.is_pending_withdraw {
                continue;
            }

            let product = contract.get_product(&product_id);
            let (balance, partition_index) = jar.get_liquid_balance(&product.terms);

            if balance > 0 {
                total_mature_balance += balance;
                total_fee += product.calculate_fee(balance);
                partition_indices.push((product_id.clone(), partition_index));
            }
        }

        let deposit = DepositDto::new(self.ticket.product_id.clone(), total_mature_balance, self.target_amount);

        let withdrawal_amount = total_mature_balance - deposit.amount;
        let withdrawal = if withdrawal_amount > 0 {
            Some(WithdrawalDto {
                amount: withdrawal_amount,
                fee: (total_fee * withdrawal_amount).div_ceil(total_mature_balance),
            })
        } else {
            None
        };

        Request {
            account_id: self.account_id.clone(),
            withdrawal,
            deposit,
            partitions: partition_indices,
        }
    }
}

impl From<&Request> for RestakeAllData {
    fn from(value: &Request) -> Self {
        let from = value
            .partitions
            .iter()
            .map(|(product_id, _)| product_id.clone())
            .collect();

        Self {
            timestamp: env::block_timestamp_ms(),
            from,
            into: value.deposit.product_id.clone(),
            restaked: value.deposit.amount.into(),
            withdrawn: value.withdrawal.map_or(0.into(), |w| w.amount.into()),
        }
    }
}

impl DepositDto {
    fn new(product_id: ProductId, mature_balance: TokenAmount, target_amount: Option<TokenAmount>) -> Self {
        let target_amount = target_amount.unwrap_or(mature_balance);
        require!(target_amount <= mature_balance, "Not enough funds to restake");

        Self {
            product_id,
            amount: target_amount,
        }
    }
}
