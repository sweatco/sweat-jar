#![cfg(not(test))]

use near_sdk::{env, PromiseOrValue};

use crate::{
    event::EventKind,
    ft_interface::FungibleTokenInterface,
    restake::api::api::{ext_self, RemainderTransfer, Request},
    Contract,
};

impl RemainderTransfer for Contract {
    fn transfer_remainder(&mut self, request: Request, event: EventKind) -> PromiseOrValue<()> {
        self.ft_contract()
            .ft_transfer(
                &request.account_id,
                request.withdrawal.amount,
                "withdraw_remainder",
                &self.wrap_fee(request.withdrawal.fee),
            )
            .then(ext_self::ext(env::current_account_id()).after_transfer_remainder(request, event))
            .into()
    }
}
