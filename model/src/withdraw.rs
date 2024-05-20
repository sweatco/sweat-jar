use near_sdk::{
    json_types::U128,
    near,
    serde::{Deserialize, Serialize},
    AccountId,
};

use crate::TokenAmount;

/// The `WithdrawView` struct represents the result of a deposit jar withdrawal operation.
#[derive(Debug, PartialEq)]
#[near(serializers=[borsh, json])]
pub struct WithdrawView {
    /// The amount of tokens that has been transferred to the user's account as part of the withdrawal.
    pub withdrawn_amount: U128,

    /// The possible fee that a user must pay for withdrawal, if it's defined by the associated Product.
    pub fee: U128,
}

#[derive(Debug, Default)]
#[near(serializers=[borsh, json])]
pub struct BulkWithdrawView {
    pub total_amount: U128,
    pub jars: Vec<WithdrawView>,
}

impl WithdrawView {
    #[must_use]
    pub fn new(amount: TokenAmount, fee: Option<Fee>) -> Self {
        let (withdrawn_amount, fee) = fee.map_or((amount, 0), |fee| (amount - fee.amount, fee.amount));

        Self {
            withdrawn_amount: U128(withdrawn_amount),
            fee: U128(fee),
        }
    }
}

#[cfg(test)]
mod test {
    use near_sdk::json_types::U128;

    use crate::withdraw::{Fee, WithdrawView};

    #[test]
    fn withdrawal_view() {
        let fee = WithdrawView::new(
            1_000_000,
            Some(Fee {
                beneficiary_id: "account_id".to_string().try_into().unwrap(),
                amount: 100,
            }),
        );

        assert_eq!(
            fee,
            WithdrawView {
                withdrawn_amount: U128(1_000_000 - 100),
                fee: U128(100),
            }
        );
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Fee {
    pub beneficiary_id: AccountId,
    pub amount: TokenAmount,
}
