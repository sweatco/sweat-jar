pub mod api;
pub mod model;
pub mod view;

#[cfg(test)]
mod helpers {
    use near_sdk::AccountId;

    use crate::{
        common::{Timestamp, TokenAmount},
        jar::model::{Jar, JarState},
        product::model::ProductId,
    };

    impl Jar {
        pub(crate) fn generate(index: u32, account_id: &AccountId, product_id: &ProductId) -> Jar {
            Self {
                index,
                account_id: account_id.clone(),
                product_id: product_id.clone(),
                created_at: 0,
                principal: 0,
                cache: None,
                claimed_balance: 0,
                is_pending_withdraw: false,
                state: JarState::Active,
                is_penalty_applied: false,
            }
        }

        pub(crate) fn principal(mut self, principal: TokenAmount) -> Jar {
            self.principal = principal;
            self
        }

        pub(crate) fn created_at(mut self, created_at: Timestamp) -> Jar {
            self.created_at = created_at;
            self
        }
    }
}
