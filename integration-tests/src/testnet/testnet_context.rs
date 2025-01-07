use anyhow::Result;
use near_workspaces::{Account, Contract};
use sweat_jar_model::api::SweatJarContract;
use sweat_model::SweatContract;

use crate::testnet::testnet_helpers::{acc_with_name, jar_testnet_contract, token_testnet_contract};

pub struct TestnetContext {
    token_contract: Contract,

    pub manager: Account,
    pub user: Account,

    jar_contract: Contract,
}

impl TestnetContext {
    pub async fn new() -> Result<Self> {
        Self::custom(
            "user_load_oracle_testing.testnet",
            "jar_contract_load_oracle_testing.testnet",
        )
        .await
    }

    pub async fn custom(user: &str, jar: &str) -> Result<Self> {
        let worker = near_workspaces::testnet().await?;

        let user = acc_with_name(user, &worker).await?;
        let manager = acc_with_name("bob_account.testnet", &worker).await?;
        let token_contract = token_testnet_contract(&worker).await?;

        let jar_contract = jar_testnet_contract(&worker, jar).await?;

        Ok(Self {
            token_contract,
            manager,
            user,
            jar_contract,
        })
    }

    pub fn jar_contract(&self) -> SweatJarContract<'_> {
        SweatJarContract {
            contract: &self.jar_contract,
        }
    }

    pub fn token_contract(&self) -> SweatContract<'_> {
        SweatContract {
            contract: &self.token_contract,
        }
    }
}
