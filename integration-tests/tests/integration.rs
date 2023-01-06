use near_contract_standards::non_fungible_token::Token;
use near_sdk::{
    serde::{Deserialize, Serialize},
    ONE_NEAR,
};
use near_units::parse_near;
use serde_json::json;
use workspaces::{network::Sandbox, Account, Contract, Worker};

use crate::utils::assert_aprox_eq;

mod utils;

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
        .create_subaccount("alice")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;
    let bob = account
        .create_subaccount("bob")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;

    account
        .call(contract.id(), "new")
        .args_json(json!({ "owner_id": account.id() }))
        .transact()
        .await?
        .into_result()?;
    account
        .call(nft_contract.id(), "new_default_meta")
        .args_json(json!({ "owner_id": account.id() }))
        .transact()
        .await?
        .into_result()?;
    account
        .call(nft_contract.id(), "nft_mint")
        .args_json(
            json!({ "token_id": "test", "receiver_id": alice.id(), "token_metadata": {"title": "Test"}}),
        )
        .deposit(parse_near!("0.1 N"))
        .transact()
        .await?
        .into_result()?;

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
    contract_addr: String,
    token_id: String,
    lender_id: String,
    borrower_id: String,
    approval_id: u64,
    expiration: u64,
    price: u128,
    state: LeaseState,
}

#[tokio::test]
async fn test_claim_back_success() -> anyhow::Result<()> {
    let context = init().await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.contract;
    let nft_contract = context.nft_contract;
    let worker = context.worker;
    let token_id = "test";
    let price = ONE_NEAR;
    let latest_block = worker.view_block().await?;
    let expiration_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 10;

    println!("Creating lease ...");
    lender
        .call(nft_contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": token_id,
            "account_id": contract.id(),
            "msg": json!({"contract_addr": nft_contract.id(),
                          "token_id": token_id,
                          "borrower_id": borrower.id(),
                          "expiration": expiration_ts_nano,
                          "price": price.to_string(),
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ Lease created");

    println!("Confirming the created lease ...");
    let leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(leases.len(), 1);

    let lease = &leases[0].1;

    assert_eq!(lease.contract_addr, nft_contract.id().to_string());
    assert_eq!(lease.token_id, "test".to_string());
    assert_eq!(lease.lender_id, lender.id().to_string());
    assert_eq!(lease.borrower_id, borrower.id().to_string());
    assert_eq!(lease.expiration, expiration_ts_nano);
    assert_eq!(lease.price, price);
    assert_eq!(lease.state, LeaseState::Pending);
    println!("      ✅ Lease creation confirmed");

    println!("Accepting the created lease ...");
    let lease_id = &leases[0].0;
    borrower
        .call(contract.id(), "lending_accept")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .deposit(price)
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ Lease accepted");

    println!("Confirm the lease is activated ...");
    let borrower_id_result: String = borrower
        .call(contract.id(), "get_borrower_by_contract_and_token")
        .args_json(json!({
            "contract_id": nft_contract.id(),
            "token_id": token_id,
        }))
        .transact()
        .await?
        .json()?;

    assert_eq!(borrower.id().to_string(), borrower_id_result);
    println!("      ✅ Lease activation accepted");

    // Fast foward and check expiration
    worker.fast_forward(12).await?;
    println!("Claiming back the NFT...");
    let lender_balance_before_claim_back = lender.view_account().await?.balance;
    let nft_contract_balance_before_claim_back = nft_contract.view_account().await?.balance;
    lender
        .call(contract.id(), "claim_back")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let lender_balance_after_claim_back = lender.view_account().await?.balance;
    let nft_contract_balance_after_claim_back = nft_contract.view_account().await?.balance;
    // This is based on the demo NFT royalty logic: the NFT contract always keep 5% for itself.
    // So the lender get the rest 95% of the rent.
    assert_aprox_eq(
        lender_balance_after_claim_back - lender_balance_before_claim_back,
        price / 20 * 19,
    );

    assert_aprox_eq(
        nft_contract_balance_after_claim_back - nft_contract_balance_before_claim_back,
        price / 20,
    );
    println!("      ✅ Royalty splits are correct");

    let owned_tokens: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json(json!({"account_id": lender.id().to_string()}))
        .transact()
        .await?
        .json()?;

    let nft_token = &owned_tokens[0];
    assert_eq!(nft_token.token_id, token_id);

    println!("      ✅ NFT claimed back");
    Ok(())
}

// Alice creates a lease to Bob.
// Bob can accept the lease for the first time
// but he should fail if he attempts to accept it for multipe times
#[tokio::test]
async fn test_accept_leases_already_lent() -> anyhow::Result<()> {
    let context = init().await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.contract;
    let nft_contract = context.nft_contract;
    let worker = context.worker;
    let token_id = "test";
    let latest_block = worker.view_block().await?;
    let expiration_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 10;

    println!("Creating lease ...");
    lender
        .call(nft_contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": token_id,
            "account_id": contract.id(),
            "msg": json!({"contract_addr": nft_contract.id(),
                          "token_id": token_id,
                          "borrower_id": borrower.id(),
                          "expiration": expiration_ts_nano,
                          "price": "1"
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ Lease created and pending on Bob's acceptence");

    // Confirming the created lease ...
    let leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;

    let lease_id = &leases[0].0;
    borrower
        .call(contract.id(), "lending_accept")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .deposit(1)
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let leases_updated: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(leases_updated[0].1.state, LeaseState::Active);
    println!("      ✅ Lease accepted by Bob");

    // Bob tries to accept the lease again.
    // This action should fail
    // TODO(haichen): make lending_accept fail explicitly
    let double_accept_result = borrower
        .call(contract.id(), "lending_accept")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .deposit(1)
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(double_accept_result.is_err());
    println!("      ✅ Lease cannot be accepted by Bob again.");
    Ok(())
}

#[tokio::test]
async fn test_accept_lease_fails_already_transferred() -> anyhow::Result<()> {
    let context = init().await?;
    let lender = context.lender;
    let borrower = context.borrower;

    let worker = workspaces::sandbox().await?;
    let account = worker.dev_create_account().await?;
    let new_owner = account
        .create_subaccount("charles")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;

    let contract = context.contract;
    let nft_contract = context.nft_contract;
    let worker = context.worker;
    let token_id = "test";
    let latest_block = worker.view_block().await?;
    let expiration_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 10;

    println!("Creating lease ...");
    lender
        .call(nft_contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": token_id,
            "account_id": contract.id(),
            "msg": json!({"contract_addr": nft_contract.id(),
                          "token_id": token_id,
                          "borrower_id": borrower.id(),
                          "expiration": expiration_ts_nano,
                          "price": "1"
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ Lease created and pending on Bob's acceptence");

    // lender Alice transfers the NFT to another user Charlse
    lender
        .call(nft_contract.id(), "nft_transfer")
        .args_json(json!({
            "receiver_id": new_owner.id(),
            "token_id": token_id,
            "approval_id": null,
            "memo": null,
        }))
        .deposit(1)
        .transact()
        .await?
        .into_result()?;

    // Verify the ownership of the token has been updated
    let token: Token = nft_contract
        .view("nft_token")
        .args_json(json!({
            "token_id": token_id,
        }))
        .await?
        .json()?;
    assert_eq!(token.owner_id.to_string(), new_owner.id().to_string());
    println!("       ✅ Lease token has been transferred from lender Alice to Charles");

    // Confirming the created lease ...
    let leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;

    let lease_id = &leases[0].0;
    let result = borrower
        .call(contract.id(), "lending_accept")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .deposit(1)
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(result.is_err());
    println!("       ✅ Lease cannot be accepted by Bob. The transaction will panic.");

    let updated_leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(updated_leases[0].1.state, LeaseState::Pending);
    println!("       ✅ Lease cannot be accepted by Bob, the state of the lease is still pending");
    Ok(())
}

// TODO: claim_back - NFT transfer check
// TODO: claim_back - check lease amount recieval, probably by using ft_balance_of().
// TODO: nft_on_approve - check lease createion happened correctly & all indices have been updated accordingly
