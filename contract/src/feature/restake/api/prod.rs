#![cfg(not(test))]

use near_sdk::{env, env::panic_str, PromiseOrValue};

use crate::{
    common::event::EventKind,
    feature::{
        ft_interface::FungibleTokenInterface,
        restake::api::{ext_self, RemainderTransfer, Request},
    },
    Contract,
};

impl RemainderTransfer for Contract {
    fn transfer_remainder(&mut self, request: Request, event: EventKind) -> PromiseOrValue<()> {
        let amount = request
            .withdrawal
            .map_or_else(|| panic_str("Transfer amount must be provided"), |w| w.net_amount());

        self.ft_contract()
            .ft_transfer(&request.account_id, amount, "withdraw_remainder")
            .then(ext_self::ext(env::current_account_id()).after_transfer_remainder(request, event))
            .into()
    }
}
