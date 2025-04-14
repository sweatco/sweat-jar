use near_sdk::{env, json_types::Base64VecU8, require};

use crate::{data::jar::Deposit, TokenAmount};

use super::{Product, Terms, WithdrawalFee};

pub trait TermsApi {
    fn allows_early_withdrawal(&self) -> bool;
    fn is_liquid(&self, deposit: &Deposit) -> bool;
}

impl TermsApi for Terms {
    fn allows_early_withdrawal(&self) -> bool {
        matches!(self, Terms::Flexible(_))
    }

    fn is_liquid(&self, deposit: &Deposit) -> bool {
        let now = env::block_timestamp_ms();
        match self {
            Terms::Fixed(terms) => deposit.is_liquid(now, terms.lockup_term.0),
            Terms::Flexible(_) => true,
            Terms::ScoreBased(terms) => deposit.is_liquid(now, terms.lockup_term.0),
        }
    }
}

pub trait ProductModelApi {
    fn get_public_key(self) -> Option<Vec<u8>>;
    fn set_public_key(&mut self, public_key: Option<Base64VecU8>);
    fn calculate_fee(&self, principal: TokenAmount) -> TokenAmount;
}

impl ProductModelApi for Product {
    fn get_public_key(self) -> Option<Vec<u8>> {
        self.public_key.map(|key| key.0)
    }

    fn set_public_key(&mut self, public_key: Option<Base64VecU8>) {
        self.public_key = public_key.map(Into::into);
    }

    fn calculate_fee(&self, principal: TokenAmount) -> TokenAmount {
        if let Some(fee) = self.withdrawal_fee.clone() {
            return match fee {
                WithdrawalFee::Fix(amount) => amount.0,
                WithdrawalFee::Percent(percent) => percent * principal,
            };
        }

        0
    }
}

pub trait ProductAssertions {
    fn assert_cap_order(&self);
    fn assert_cap(&self, amount: TokenAmount);
    fn assert_enabled(&self);
    fn assert_fee_amount(&self);
    fn assert_score_based_product_is_protected(&self);
}

impl ProductAssertions for Product {
    fn assert_cap_order(&self) {
        require!(self.cap.min() < self.cap.max(), "Cap minimum must be less than maximum");
    }

    fn assert_cap(&self, amount: TokenAmount) {
        if self.cap.min() > amount || amount > self.cap.max() {
            env::panic_str(&format!(
                "Total amount is out of product bounds: [{}..{}]",
                self.cap.min(),
                self.cap.max()
            ));
        }
    }

    fn assert_enabled(&self) {
        require!(
            self.is_enabled,
            "It's not possible to create new jars for this product: the product is disabled."
        );
    }

    /// Check if fee in new product is not too high
    fn assert_fee_amount(&self) {
        let Some(ref fee) = self.withdrawal_fee else {
            return;
        };

        let fee_ok = match fee {
            WithdrawalFee::Fix(amount) => amount.0 < self.cap.min(),
            WithdrawalFee::Percent(percent) => percent.to_f32() < 100.0,
        };

        require!(
            fee_ok,
            "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
        );
    }

    fn assert_score_based_product_is_protected(&self) {
        if matches!(self.terms, Terms::ScoreBased(_)) {
            require!(self.public_key.is_some(), "Score based must be protected.");
        }
    }
}