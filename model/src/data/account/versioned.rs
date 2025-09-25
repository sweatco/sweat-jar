use std::{
    io::{Error, ErrorKind::InvalidData, Read},
    ops::{Deref, DerefMut},
};

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env::{self, panic_str},
};

use crate::{
    data::{
        account::v1::AccountV1,
        jar::{Deposit, Jar},
        product::{Product, ProductId},
    },
    interest::InterestCalculator,
    AccountScore, Timestamp, Timezone, TokenAmount,
};

use super::{Account, AccountCompanion};

#[derive(BorshSerialize, Debug, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub enum AccountVersioned {
    V1(AccountV1),
    V1Sorted(AccountV1),
    V1SortedAndMerged(AccountV1),
}

impl AccountVersioned {
    pub fn new(account: AccountV1) -> Self {
        AccountVersioned::V1SortedAndMerged(account)
    }
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to the latest version
impl BorshDeserialize for AccountVersioned {
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let tag: u8 = BorshDeserialize::deserialize_reader(reader)?;

        let result: AccountV1 = match tag {
            0 | 1 => {
                let account: AccountV1 = BorshDeserialize::deserialize_reader(reader)?;
                account.with_merged_deposits()
            }
            2 => BorshDeserialize::deserialize_reader(reader)?,
            // Add new versions here:
            _ => return Err(Error::new(InvalidData, format!("Unexpected variant tag: {tag:?}"))),
        };

        Ok(AccountVersioned::V1SortedAndMerged(result))
    }
}

impl Default for AccountVersioned {
    fn default() -> Self {
        Self::V1SortedAndMerged(AccountV1::default())
    }
}

impl Deref for AccountVersioned {
    type Target = AccountV1;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1SortedAndMerged(account) => account,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(account) => account, <- Add a new version here
            _ => panic!("Cannot deref this variant directly. Use V1SortedAndMerged."),
        }
    }
}

impl DerefMut for AccountVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V1SortedAndMerged(account) => account,
            // Guaranteed by `BorshDeserialize` implementation
            // Self::V2(account) => account, <- Add a new version here
            _ => panic!("Cannot deref this variant directly. Use V1SortedAndMerged."),
        }
    }
}

impl Account {
    #[must_use]
    pub fn with_sorted_deposits(&self) -> Self {
        let mut jars = self.jars.clone();
        for jar in jars.values_mut() {
            jar.sort_deposits();
        }

        Self { jars, ..self.clone() }
    }

    #[must_use]
    pub fn with_merged_deposits(&self) -> Self {
        let mut jars = self.jars.clone();
        for jar in jars.values_mut() {
            jar.merge_deposits();
            jar.sort_deposits();
        }

        Self { jars, ..self.clone() }
    }

    pub fn get_total_principal(&self) -> TokenAmount {
        self.jars
            .iter()
            .fold(TokenAmount::default(), |acc, (_, jar)| acc + jar.total_principal())
    }

    pub fn get_jar(&self, product_id: &ProductId) -> &Jar {
        self.jars
            .get(product_id)
            .unwrap_or_else(|| panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub fn get_jar_mut(&mut self, product_id: &ProductId) -> &mut Jar {
        self.jars
            .get_mut(product_id)
            .unwrap_or_else(|| panic_str(format!("Jar for product {product_id} is not found").as_str()))
    }

    pub fn deposit(&mut self, product_id: &ProductId, principal: TokenAmount, time: Option<Timestamp>) {
        let deposit = Deposit::new(time.unwrap_or_else(env::block_timestamp_ms), principal);
        let jar = self.jars.entry(product_id.clone()).or_default();
        jar.deposits.push(deposit);
    }

    pub fn try_set_timezone(&mut self, timezone: Option<Timezone>) {
        if self.score.is_timezone_set() {
            return;
        }

        if let Some(timezone) = timezone {
            self.score = AccountScore::new(timezone);
        } else {
            panic_str("Trying to create score based jar without providing time zone");
        }
    }

    pub fn apply(&mut self, companion: &AccountCompanion) {
        if let Some(nonce) = companion.nonce {
            self.nonce = nonce;
        }

        if let Some(jars) = &companion.jars {
            for (product_id, jar_companion) in jars {
                let jar = self.jars.get_mut(product_id).expect("Jar is not found");
                jar.apply(jar_companion);
            }
        }

        if let Some(score) = companion.score {
            self.score = score;
        }

        if let Some(is_penalty_applied) = companion.is_penalty_applied {
            self.is_penalty_applied = is_penalty_applied;
        }
    }

    pub fn update_jar_cache(&mut self, product: &Product, now: Timestamp) {
        let jar = self.get_jar(&product.id);
        let (interest, remainder) = product.terms.get_interest(self, jar, now);
        self.get_jar_mut(&product.id).update_cache(interest, remainder, now);
    }

    pub fn has_score_jars(&self) -> bool {
        self.score.is_timezone_set()
    }
}
