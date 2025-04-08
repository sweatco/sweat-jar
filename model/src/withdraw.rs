use near_sdk::{json_types::U128, near, AccountId};

use crate::{ProductId, TokenAmount};

/// The `WithdrawView` struct represents the result of a deposit jar withdrawal operation.
#[derive(Debug, PartialEq)]
#[near(serializers=[json])]
pub struct WithdrawView {
    pub product_id: ProductId,
    /// The amount of tokens that has been transferred to the user's account as part of the withdrawal.
    pub withdrawn_amount: U128,

    /// The possible fee that a user must pay for withdrawal, if it's defined by the associated Product.
    pub fee: U128,
}

#[derive(Debug, Default)]
#[near(serializers=[json])]
// TODO: doc change
pub struct BulkWithdrawView {
    pub total_amount: U128,
    pub withdrawals: Vec<WithdrawView>,
}

impl WithdrawView {
    #[must_use]
    pub fn new(product_id: &ProductId, amount: TokenAmount, fee: TokenAmount) -> Self {
        let net_amount = amount - fee;

        Self {
            product_id: product_id.clone(),
            withdrawn_amount: net_amount.into(),
            fee: U128(fee),
        }
    }
}

#[cfg(test)]
mod test {
    use near_sdk::json_types::U128;

    use crate::{withdraw::WithdrawView, ProductId};

    #[test]
    fn withdrawal_view() {
        let fee = WithdrawView::new(&ProductId::new(), 1_000_000, 100);

        assert_eq!(
            fee,
            WithdrawView {
                product_id: ProductId::new(),
                withdrawn_amount: U128(1_000_000 - 100),
                fee: U128(100),
            }
        );
    }
}

#[derive(Clone, Debug)]
#[near(serializers=[json])]
pub struct Fee {
    pub beneficiary_id: AccountId,
    pub amount: TokenAmount,
}
