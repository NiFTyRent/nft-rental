use near_sdk::serde::{Deserialize, Serialize};
use near_units::parse_near;
use serde_json::json;
use std::{env, fs};
use tracing::info;
use workspaces::prelude::*;
use workspaces::{network::Sandbox, Account, Contract, Worker};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wasm_arg: &str = &(env::args().nth(1).unwrap());
    let nft_wasm_arg: &str = &(env::args().nth(2).unwrap());
    let wasm_filepath = fs::canonicalize(env::current_dir()?.join(wasm_arg))?;
    let nft_wasm_filepath = fs::canonicalize(env::current_dir()?.join(nft_wasm_arg))?;

    let worker = workspaces::sandbox().await?;
    let wasm = std::fs::read(wasm_filepath)?;
    let nft_wasm = std::fs::read(nft_wasm_filepath)?;
    let contract = worker.dev_deploy(&wasm).await?;
    let nft_contract = worker.dev_deploy(&nft_wasm).await?;

    // create accounts
    let account = worker.dev_create_account().await?;
    let alice = account
        .create_subaccount(&worker, "alice")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;
    let bob = account
        .create_subaccount(&worker, "bob")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;

    account
        .call(&worker, contract.id(), "new")
        .args_json(json!({ "owner_id": account.id() }))?
        .transact()
        .await?;
    account
        .call(&worker, nft_contract.id(), "new_default_meta")
        .args_json(json!({ "owner_id": account.id() }))?
        .transact()
        .await?;
    account
        .call(&worker, nft_contract.id(), "nft_mint")
        .args_json(
            json!({ "token_id": "test", "receiver_id": alice.id(), "token_metadata": {"title": "Test"}}),
        )?
        .deposit(parse_near!("0.1 N"))
        .transact()
        .await?;

    // begin tests
    test_create_lease(&alice, &bob, &contract, &nft_contract, &worker).await?;
    // test_changes_message(&alice, &bob, &contract, &nft_contract, &worker).await?;
    Ok(())
}

// TODO(libo): we can import them from the contract under testing.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
enum LeaseState {
    Pending,
    Active,
    Expired,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
struct LeaseCondition {
    contract_addr: String, // NFT contract
    token_id: String,      // NFT token
    owner_id: String,      // Owner of the NFT
    borrower: String,      // Borrower of the NFT
    approval_id: u64,      // Approval from owner to lease
    expiration: u64,       // TODO: duration
    amount_near: u128,     // proposed lease cost
    state: LeaseState,     // current lease state
}

async fn test_create_lease(
    lender: &Account,
    borrower: &Account,
    contract: &Contract,
    nft_contract: &Contract,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<()> {
    lender
        .call(&worker, nft_contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": "test",
            "account_id": contract.id(),
            "msg": json!({"contract_addr": nft_contract.id(),
                          "token_id": "test",
                          "borrower": borrower.id(),
                          "expiration": 3600,
                          "amount_near": "1"
            }).to_string()
        }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    let leases: Vec<(String, LeaseCondition)> = contract
        .call(&worker, "leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))?
        .transact()
        .await?
        .json()?;
    assert_eq!(leases.len(), 1);

    let lease = &leases[0].1;

    assert_eq!(lease.contract_addr, nft_contract.id().to_string());
    assert_eq!(lease.token_id, "test".to_string());
    assert_eq!(lease.owner_id, lender.id().to_string());
    assert_eq!(lease.borrower, borrower.id().to_string());
    assert_eq!(lease.expiration, 3600);
    assert_eq!(lease.amount_near, 1);
    assert_eq!(lease.state, LeaseState::Pending);

    println!("      Passed ✅ create lease");
    Ok(())
}

async fn test_changes_message(
    user: &Account,
    borrower: &Account,
    contract: &Contract,
    nft_contract: &Contract,
    worker: &Worker<Sandbox>,
) -> anyhow::Result<()> {
    user.call(&worker, contract.id(), "set_greeting")
        .args_json(json!({"message": "Howdy"}))?
        .transact()
        .await?;

    let message: String = user
        .call(&worker, contract.id(), "get_greeting")
        .args_json(json!({}))?
        .transact()
        .await?
        .json()?;

    assert_eq!(message, "Howdy".to_string());
    println!("      Passed ✅ changes message");
    Ok(())
}
