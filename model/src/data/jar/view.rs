use std::collections::HashMap;

use near_sdk::{json_types::U128, near, Timestamp};

use crate::{
    data::{account::Account, product::ProductId},
    TokenAmount,
};

#[near(serializers=[json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct JarsView(pub HashMap<ProductId, Vec<(Timestamp, U128)>>);

pub struct DepositView(ProductId, Timestamp, TokenAmount);

impl DepositView {
    pub fn product_id(&self) -> ProductId {
        self.0.clone()
    }

    pub fn timestamp(&self) -> Timestamp {
        self.1
    }

    pub fn principal(&self) -> TokenAmount {
        self.2
    }
}

impl JarsView {
    pub fn get_total_deposits_number(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }

    pub fn get_total_principal(&self) -> TokenAmount {
        self.0
            .values()
            .map(|deposits| deposits.iter().map(|(_, principal)| principal.0).sum::<TokenAmount>())
            .sum()
    }

    pub fn get_last_deposit(&self) -> Option<DepositView> {
        self.list_deposits().into_iter().max_by_key(DepositView::timestamp)
    }

    pub fn get_first_deposit(&self) -> Option<DepositView> {
        self.list_deposits().into_iter().min_by_key(DepositView::timestamp)
    }

    pub fn get_total_principal_for_product(&self, product_id: &ProductId) -> TokenAmount {
        self.0
            .get(product_id)
            .expect("Product not found")
            .iter()
            .map(|(_, principal)| principal.0)
            .sum()
    }

    pub fn get_principal_per_product(&self) -> Vec<TokenAmount> {
        self.0
            .values()
            .map(|deposits| deposits.iter().map(|(_, principal)| principal.0).sum::<TokenAmount>())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn list_deposits(&self) -> Vec<DepositView> {
        self.0
            .iter()
            .flat_map(|(product_id, deposits)| {
                deposits
                    .iter()
                    .map(move |(timestamp, principal)| DepositView(product_id.clone(), *timestamp, principal.0))
            })
            .collect()
    }
}

impl From<&Account> for JarsView {
    fn from(account: &Account) -> Self {
        Self(
            account
                .jars
                .iter()
                .map(|(product_id, jar)| {
                    (
                        product_id.clone(),
                        jar.deposits
                            .iter()
                            .map(|deposit| (deposit.created_at, deposit.principal.into()))
                            .collect(),
                    )
                })
                .collect(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
#[near(serializers=[json])]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<ProductId, U128>,
    pub total: U128,
}

impl Default for AggregatedTokenAmountView {
    fn default() -> Self {
        Self {
            detailed: HashMap::default(),
            total: U128(0),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
#[near(serializers=[json])]
pub struct AggregatedInterestView {
    pub amount: AggregatedTokenAmountView,
    pub timestamp: Timestamp,
}
