use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::U128;
use near_units::parse_near;
use nft_rental::{LeaseCondition, LeaseState};
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
    ft_contract: Contract,
    worker: Worker<Sandbox>,
}

const CONTRACT_CODE: &[u8] =
    include_bytes!("../../contract/target/wasm32-unknown-unknown/release/nft_rental.wasm");
const NFT_PAYOUT_CODE: &[u8] =
    include_bytes!("../target/wasm32-unknown-unknown/release/test_nft_with_payout.wasm");
const NFT_NO_PAYOUT_CODE: &[u8] =
    include_bytes!("../target/wasm32-unknown-unknown/release/test_nft_without_payout.wasm");
const FT_CODE: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/test_ft.wasm");

async fn init(nft_code: &[u8]) -> anyhow::Result<Context> {
    let worker = workspaces::sandbox().await?;
    let contract = worker.dev_deploy(CONTRACT_CODE).await?;
    let nft_contract = worker.dev_deploy(nft_code).await?;
    let ft_contract = worker.dev_deploy(FT_CODE).await?;

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
        .call(nft_contract.id(), "new")
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
    account
        .call(ft_contract.id(), "new")
        .args_json(json!({ "owner_id": ft_contract.id(), "total_supply": "10000000000" }))
        .transact()
        .await?
        .into_result()?;

    account
        .call(ft_contract.id(), "unsafe_register_and_deposit")
        .args_json(json!({ "account_id": contract.id(), "balance": 10000000}))
        .transact()
        .await?
        .into_result()?;

    account
        .call(ft_contract.id(), "unsafe_register_and_deposit")
        .args_json(json!({ "account_id": alice.id(), "balance": 10000000}))
        .transact()
        .await?
        .into_result()?;

    account
        .call(ft_contract.id(), "unsafe_register_and_deposit")
        .args_json(json!({ "account_id": bob.id(), "balance": 10000000}))
        .transact()
        .await?
        .into_result()?;

    account
        .call(ft_contract.id(), "unsafe_register_and_deposit")
        .args_json(json!({ "account_id": nft_contract.id(), "balance": 10000000}))
        .transact()
        .await?
        .into_result()?;

    Ok(Context {
        lender: alice,
        borrower: bob,
        contract,
        nft_contract,
        ft_contract,
        worker,
    })
}

#[tokio::test]
async fn test_claim_back_with_payout_success() -> anyhow::Result<()> {
    let context = init(NFT_PAYOUT_CODE).await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test";
    let price = 10000;
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
                          "ft_contract_addr": ft_contract.id(),
                          "expiration": expiration_ts_nano,
                          "price": price.to_string(),
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
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

    assert_to_string_eq!(lease.contract_addr, nft_contract.id());
    assert_eq!(lease.token_id, "test".to_string());
    assert_to_string_eq!(lease.lender_id, lender.id().to_string());
    assert_to_string_eq!(lease.borrower_id, borrower.id().to_string());
    assert_eq!(lease.expiration, expiration_ts_nano);
    assert_eq!(lease.price, price);
    assert_eq!(lease.state, LeaseState::Pending);
    println!("      ✅ Lease creation confirmed");

    println!("Accepting the created lease ...");
    let lease_id = &leases[0].0;
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id(),
            "amount": price.to_string(),
            "memo": "",
            "msg": json!({
                "lease_id": lease_id,
            }).to_string()
        }))
        .deposit(1)
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
    let lender_balance_before_claim_back: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;

    let nft_contract_balance_before_claim_back: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": nft_contract.id(),
        }))
        .await?
        .json()?;

    lender
        .call(contract.id(), "claim_back")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let lender_balance_after_claim_back: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;

    let nft_contract_balance_after_claim_back: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": nft_contract.id(),
        }))
        .await?
        .json()?;
    // This is based on the demo NFT royalty logic: the NFT contract always keep 5% for itself.
    // So the lender get the rest 95% of the rent.
    assert_aprox_eq(
        lender_balance_after_claim_back.0 - lender_balance_before_claim_back.0,
        price / 20 * 19,
    );
    assert_aprox_eq(
        nft_contract_balance_after_claim_back.0 - nft_contract_balance_before_claim_back.0,
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

#[tokio::test]
async fn test_claim_back_without_payout_success() -> anyhow::Result<()> {
    let context = init(NFT_NO_PAYOUT_CODE).await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test";
    let price = 10000;
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
                          "ft_contract_addr": ft_contract.id(),
                          "expiration": expiration_ts_nano,
                          "price": price.to_string(),
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
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

    println!("Accepting the created lease ...");
    let lease_id = &leases[0].0;
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id(),
            "amount": price.to_string(),
            "memo": "",
            "msg": json!({
                "lease_id": lease_id,
            }).to_string()
        }))
        .deposit(1)
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ Lease accepted");

    // Fast foward and check expiration
    worker.fast_forward(12).await?;
    println!("Claiming back the NFT...");
    let lender_balance_before_claim_back: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;
    lender
        .call(contract.id(), "claim_back")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let lender_balance_after_claim_back: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;
    // All fund goes to the lender when no payout split.
    assert_aprox_eq(
        lender_balance_after_claim_back.0 - lender_balance_before_claim_back.0,
        price,
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
    let context = init(NFT_PAYOUT_CODE).await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
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
                          "ft_contract_addr": ft_contract.id(),
                          "expiration": expiration_ts_nano,
                          "price": "1000"
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
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
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id(),
            "amount": "1000",
            "memo": "",
            "msg": json!({
                "lease_id": lease_id,
            }).to_string()
        }))
        .deposit(1)
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let borrower_balance_after_first_accept: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": borrower.id(),
        }))
        .await?
        .json()?;
    assert_eq!(borrower_balance_after_first_accept.0, 10000000 - 1000);

    let leases_updated: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(leases_updated[0].1.state, LeaseState::Active);
    println!("      ✅ Lease accepted by Bob");

    // Bob tries to accept the lease again.
    // The transaction will be aborted and the amount will be returned to borrower
    // TODO[Libo, Haichen]: check what is the expected return if the promise chain throws an exception
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id(),
            "amount": "1000",
            "memo": "",
            "msg": json!({
                "lease_id": lease_id,
            }).to_string()
        }))
        .deposit(1)
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let borrower_balance_after_second_accept: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": borrower.id(),
        }))
        .await?
        .json()?;
    assert_eq!(borrower_balance_after_second_accept.0, 10000000 - 1000);

    println!("      ✅ Lease cannot be accepted by Bob again.");
    Ok(())
}

#[tokio::test]
async fn test_accept_lease_fails_already_transferred() -> anyhow::Result<()> {
    let context = init(NFT_PAYOUT_CODE).await?;
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
    let ft_contract = context.ft_contract;
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
                          "ft_contract_addr": ft_contract.id(),
                          "expiration": expiration_ts_nano,
                          "price": "1000"
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
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
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id(),
            "amount": "1000",
            "memo": "",
            "msg": json!({
                "lease_id": lease_id,
            }).to_string()
        }))
        .deposit(1)
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let borrower_balance_after_accept: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": borrower.id(),
        }))
        .await?
        .json()?;

    assert_eq!(borrower_balance_after_accept.0, 10000000);
    println!("       ✅ Lease cannot be accepted by Bob. The transaction will be aborted and Bos's balance will not change.");

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
// TODO: add a dummy NFT contract without payout being implemented to test the related scenarios
