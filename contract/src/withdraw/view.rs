use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
};

use crate::{common::TokenAmount, ft_interface::Fee};

/// The `WithdrawView` struct represents the result of a deposit jar withdrawal operation.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct WithdrawView {
    /// The amount of tokens that has been transferred to the user's account as part of the withdrawal.
    withdrawn_amount: U128,

    /// The possible fee that a user must pay for withdrawal, if it's defined by the associated Product.
    fee: U128,
}

impl WithdrawView {
    pub(crate) fn new(amount: TokenAmount, fee: Option<Fee>) -> Self {
        let (withdrawn_amount, fee) = fee.map_or((amount, 0), |fee| (amount - fee.amount, fee.amount));

        Self {
            withdrawn_amount: U128(withdrawn_amount),
            fee: U128(fee),
        }
    }
}
