use std::{
    process::{Command, Stdio},
    sync::Mutex,
};

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde_json::Value;
use workspaces::Account;

use crate::{context::Context, product::RegisterProductCommand};

pub trait ValueGetters {
    fn get_u128(&self, key: &str) -> u128;
    fn get_interest(&self) -> u128;
    fn get_jar_id(&self) -> String;
}

impl ValueGetters for Value {
    fn get_u128(&self, key: &str) -> u128 {
        self.as_object()
            .unwrap()
            .get(key)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
            .parse::<u128>()
            .unwrap()
    }

    fn get_interest(&self) -> u128 {
        self.as_object().unwrap().get("amount").unwrap().get_u128("total")
    }

    fn get_jar_id(&self) -> String {
        self.as_object()
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }
}

static CONTRACT_READY: Mutex<bool> = Mutex::new(false);

/// Compile contract in release mode and prepare it for integration tests usage
pub fn build_contract() -> anyhow::Result<()> {
    let mut ready = CONTRACT_READY.lock().unwrap();

    if *ready {
        return Ok(());
    }

    Command::new("make")
        .arg("build")
        .current_dir("..")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    *ready = true;

    Ok(())
}

pub(crate) struct Prepared {
    pub(crate) context: Context,
    pub(crate) manager: Account,
    pub(crate) alice: Account,
    pub(crate) fee_account: Account,
}

pub(crate) async fn prepare_contract(
    products: impl IntoIterator<Item = RegisterProductCommand>,
) -> anyhow::Result<Prepared> {
    let mut context = Context::new().await?;

    let manager = context.account("manager").await?;
    let alice = context.account("alice").await?;
    let fee_account = context.account("fee").await?;

    context.ft_contract.init().await?;
    context
        .jar_contract
        .init(context.ft_contract.account(), &fee_account, manager.id())
        .await?;

    context
        .ft_contract
        .storage_deposit(context.jar_contract.account())
        .await?;

    context.ft_contract.storage_deposit(&fee_account).await?;
    context.ft_contract.storage_deposit(&alice).await?;
    context.ft_contract.mint_for_user(&alice, 100_000_000).await?;

    for product in products {
        context.jar_contract.register_product(&manager, product.json()).await?;
    }

    Ok(Prepared {
        context,
        manager,
        alice,
        fee_account,
    })
}

pub(crate) fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key: VerifyingKey = VerifyingKey::from(&signing_key);

    (signing_key, verifying_key)
}
