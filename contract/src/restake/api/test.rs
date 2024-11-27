#![cfg(test)]

use near_sdk::PromiseOrValue;

use crate::{
    event::EventKind,
    restake::api::api::{RemainderTransfer, RemainderTransferCallback, Request},
    Contract,
};

impl RemainderTransfer for Contract {
    fn transfer_remainder(&mut self, request: Request, event: EventKind) -> PromiseOrValue<()> {
        self.after_transfer_remainder(request, event)
    }
}
