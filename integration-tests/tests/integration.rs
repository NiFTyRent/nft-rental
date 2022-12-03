use near_contract_standards::non_fungible_token::Token;
use near_sdk::serde::{Deserialize, Serialize};
use near_units::parse_near;
use serde_json::json;
use workspaces::prelude::*;
use workspaces::{network::Sandbox, Account, Contract, Worker};

const ONE_BLOCK_IN_NANO: u64 = 2000000000;

struct Context {
    lender: Account,
    borrower: Account,
    contract: Contract,
    nft_contract: Contract,
    worker: Worker<Sandbox>,
}

const CONTRACT_CODE: &[u8] =
    include_bytes!("../../contract/target/wasm32-unknown-unknown/release/nft_rental.wasm");
const NFT_CODE: &[u8] =
    include_bytes!("../../demo_nft_contract/target/wasm32-unknown-unknown/release/tamagotchi.wasm");

async fn init() -> anyhow::Result<Context> {
    let worker = workspaces::sandbox().await?;
    let contract = worker.dev_deploy(CONTRACT_CODE).await?;
    let nft_contract = worker.dev_deploy(NFT_CODE).await?;

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

    Ok(Context {
        lender: alice,
        borrower: bob,
        contract,
        nft_contract,
        worker,
    })
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
#[tokio::test]
async fn test_create_lease() -> anyhow::Result<()> {
    let context = init().await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.contract;
    let nft_contract = context.nft_contract;
    let worker = context.worker;
    let token_id = "test";
    let latest_block = worker.view_latest_block().await?;
    let expiration_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 10;

    lender
        .call(&worker, nft_contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": token_id,
            "account_id": contract.id(),
            "msg": json!({"contract_addr": nft_contract.id(),
                          "token_id": token_id,
                          "borrower": borrower.id(),
                          "expiration": expiration_ts_nano,
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
    assert_eq!(lease.expiration, expiration_ts_nano);
    assert_eq!(lease.amount_near, 1);
    assert_eq!(lease.state, LeaseState::Pending);

    println!("      Passed ✅ create lease");

    let lease_id = &leases[0].0;
    borrower
        .call(&worker, contract.id(), "lending_accept")
        .args_json(json!({
            "lease_id": lease_id,
        }))?
        .deposit(1)
        .max_gas()
        .transact()
        .await?;

    let borrower_id_result: String = borrower
        .call(&worker, contract.id(), "get_borrower")
        .args_json(json!({
            "contract_id": nft_contract.id(),
            "token_id": token_id,
        }))?
        .transact()
        .await?
        .json()?;

    assert_eq!(borrower.id().to_string(), borrower_id_result);

    // test: post accept. fast foward and check expiration
    println!("testing post accept");
    worker.fast_forward(12).await?;
    lender
        .call(&worker, contract.id(), "claim_back")
        .args_json(json!({
            "lease_id": lease_id,
        }))?
        .max_gas()
        .transact()
        .await?;

    let owned_tokens: Vec<Token> = nft_contract
        .call(&worker, "nft_tokens_for_owner")
        .args_json(json!({
            "account_id": lender.id().to_string(), }))?
        .transact()
        .await?
        .json()?;

    let nft_token = &owned_tokens[0];
    assert_eq!(nft_token.token_id, token_id);

    println!("      Passed ✅ claim back");
    Ok(())
}

// async fn test_changes_message(
//     lender: &Account,
//     borrower: &Account,
//     contract: &Contract,
//     nft_contract: &Contract,
//     worker: &Worker<Sandbox>,
// ) -> anyhow::Result<()> {
//     let token_id = "test_accept";

//     // set up
//     lender
//         .call(&worker, nft_contract.id(), "nft_approve")
//         .args_json(json!({
//             "token_id": "test",
//             "account_id": contract.id(),
//             "msg": json!({"contract_addr": nft_contract.id(),
//                           "token_id": token_id,
//                           "borrower": borrower.id(),
//                           "expiration": 3600,
//                           "amount_near": "1"
//             }).to_string()
//         }))?
//         .deposit(parse_near!("1 N"))
//         .transact()
//         .await?;

//     let leases: Vec<(String, LeaseCondition)> = contract
//         .call(&worker, "leases_by_owner")
//         .args_json(json!({"account_id": lender.id()}))?
//         .transact()
//         .await?
//         .json()?;

//     // test, pre-accept: TODO

//     // test, accept occured
//     let lease_id = &leases[0].0;
//     borrower
//         .call(&worker, contract.id(), "lending_accept")
//         .args_json(json!({
//             "lease_id": lease_id,
//         }))?
//         .deposit(1)
//         .transact()
//         .await?;

//     let leases_by_borrower: Vec<(String, LeaseCondition)> = contract
//         .call(&worker, "leases_by_borrower")
//         .args_json(json!({"account_id": borrower.id()}))?
//         .transact()
//         .await?
//         .json()?;

//     assert_eq!(leases.len(), 1);

//     let borrower_id_result: String = borrower
//         .call(&worker, contract.id(), "get_borrower")
//         .args_json(json!({
//             "contract_id": nft_contract.id(),
//             "token_id": token_id,
//         }))?
//         .transact()
//         .await?
//         .json()?;

//     // test. rent expires

//     // let lease = &leases_by_borrower[0].1;

//     println!("{:?}", borrower_id_result);

//     println!("      Passed ✅ changes message");
// }
