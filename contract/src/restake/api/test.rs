#![cfg(test)]

use near_sdk::PromiseOrValue;

use crate::{
    restake::api::api::{RemainderTransfer, RemainderTransferCallback, Request},
    Contract,
};

impl RemainderTransfer for Contract {
    fn transfer_remainder(&mut self, request: Request) -> PromiseOrValue<()> {
        self.after_transfer_remainder(request).into()
    }
}
