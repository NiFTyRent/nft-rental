use crate::utils::assert_aprox_eq;
use anyhow::Ok;
use near_contract_standards::non_fungible_token::{
    metadata::NFTContractMetadata, metadata::NFT_METADATA_SPEC, Token,
};
use near_sdk::json_types::U128;
use near_sdk::{log, AccountId};
use near_units::parse_near;
use nft_rental::{LeaseCondition, LeaseState};
use serde_json::json;
use workspaces::{network::Sandbox, Account, Contract, Worker};

mod utils;

const ONE_BLOCK_IN_NANO: u64 = 2000000000;

struct Context {
    worker: Worker<Sandbox>,
    rental_contract: Contract,
    marketplace_contract: Contract,
    nft_contract: Contract,
    ft_contract: Contract,
    lender: Account,
    borrower: Account,
    lease_nft_receiver: Account,
    markeplace_owner: Account,
}

const CONTRACT_CODE: &[u8] =
    include_bytes!("../../contract/target/wasm32-unknown-unknown/release/nft_rental.wasm");
const MARKETPLACE_CONTRACT_CODE: &[u8] = include_bytes!(
    "../../marketplace/target/wasm32-unknown-unknown/release/niftyrent_marketplace.wasm"
);
const NFT_PAYOUT_CODE: &[u8] =
    include_bytes!("../target/wasm32-unknown-unknown/release/test_nft_with_payout.wasm");
const NFT_NO_PAYOUT_CODE: &[u8] =
    include_bytes!("../target/wasm32-unknown-unknown/release/test_nft_without_payout.wasm");
const FT_CODE: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/test_ft.wasm");

// TODO(syu): init is used by all tests, making run time too long. Consider simplify init for some tests.
async fn init(nft_code: &[u8]) -> anyhow::Result<Context> {
    log!("Initialising Test ...");

    let worker = workspaces::sandbox().await?;
    let rental_contract = worker.dev_deploy(CONTRACT_CODE).await?;
    let marketplace_contract = worker.dev_deploy(MARKETPLACE_CONTRACT_CODE).await?;
    let nft_contract = worker.dev_deploy(nft_code).await?;
    let ft_contract = worker.dev_deploy(FT_CODE).await?;

    // create accounts
    let account = worker.dev_create_account().await?;

    let alice = account
        .create_subaccount("alice")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;

    let bob = account
        .create_subaccount("bob")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;

    let charlie = account
        .create_subaccount("charlie")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;

    let marketplace_owner = account
        .create_subaccount("marketplace_owner")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;

    let treasury = account
        .create_subaccount("treasury")
        .initial_balance(parse_near!("1 N"))
        .transact()
        .await?
        .into_result()?;

    // initialise contracts
    account
        .call(rental_contract.id(), "new")
        .args_json(json!({ "owner_id": account.id() }))
        .transact()
        .await?
        .into_result()?;

    account
        .call(marketplace_contract.id(), "new")
        .args_json(json!({
           "owner_id": marketplace_owner.id(),
           "treasury_id": treasury.id(),
           "rental_contract_id": account.id(),
        }))
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
        .call(ft_contract.id(), "new")
        .args_json(json!({ "owner_id": ft_contract.id(), "total_supply": "10000000000" }))
        .transact()
        .await?
        .into_result()?;

    // mint nfts for renting
    account
        .call(nft_contract.id(), "nft_mint")
        .args_json(
            json!({ "token_id": "test", "receiver_id": alice.id(), "token_metadata": {"title": "Test"}}),
        )
        .deposit(parse_near!("0.1 N"))
        .transact()
        .await?
        .into_result()?;

    // register accounts for ft_contract and deposit
    account
        .call(ft_contract.id(), "unsafe_register_and_deposit")
        .args_json(json!({ "account_id": rental_contract.id(), "balance": 100000000}))
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
        .args_json(json!({ "account_id": charlie.id(), "balance": 10000000}))
        .transact()
        .await?
        .into_result()?;

    account
        .call(ft_contract.id(), "unsafe_register_and_deposit")
        .args_json(json!({ "account_id": nft_contract.id(), "balance": 10000000}))
        .transact()
        .await?
        .into_result()?;

    account
        .call(ft_contract.id(), "unsafe_register_and_deposit")
        .args_json(json!({ "account_id": marketplace_owner.id(), "balance": 10000000}))
        .transact()
        .await?
        .into_result()?;

    // add allowed nft contracts for marketplace
    log!("Set allowed NFT contracts for marketplace...");
    let allowed_nft_contracts_ids_expected = vec![nft_contract.id().as_str()];
    log!(format!(
        "allowing nft contracts list: {:?}",
        json!({
            "nft_contract_ids": allowed_nft_contracts_ids_expected,
        })
    ));

    let result = marketplace_owner
        .call(marketplace_contract.id(), "add_allowed_nft_contract_ids")
        .args_json(json!({
            "nft_contract_ids": allowed_nft_contracts_ids_expected,
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
        .transact()
        .await?;
    assert!(result.is_success());

    // view the allowed nft contracts
    let allowed_nft_contracts_real: Vec<AccountId> = marketplace_owner
        .call(marketplace_contract.id(), "list_allowed_nft_contract_ids")
        .max_gas()
        .transact()
        .await?
        .json()?;
    log!(format!(
        "Returned allowed nft contracts list: {:?}",
        allowed_nft_contracts_real
    ));
    assert_eq!(allowed_nft_contracts_real.len(), 1);
    assert_eq!(allowed_nft_contracts_ids_expected[0], allowed_nft_contracts_real[0].as_str());

    // add allowed ft contracts for marketplace

    Ok(Context {
        worker: worker,
        rental_contract: rental_contract,
        marketplace_contract: marketplace_contract,
        nft_contract: nft_contract,
        ft_contract: ft_contract,
        lender: alice,
        borrower: bob,
        lease_nft_receiver: charlie,
        markeplace_owner: marketplace_owner,
    })
}

#[tokio::test]
async fn test_claim_back_with_payout_success() -> anyhow::Result<()> {
    let context = init(NFT_PAYOUT_CODE).await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test";
    let price = 10000;
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
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
    assert_eq!(lease.end_ts_nano, expiration_ts_nano);
    assert_eq!(lease.price.0, price);
    assert_eq!(lease.state, LeaseState::PendingOnRent);
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
    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test";
    let price = 10000;
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
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
    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test";
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
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

    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test";
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
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
    println!("      ✅ Lease token has been transferred from lender Alice to Charles");

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
    println!("      ✅ Lease cannot be accepted by Bob. The transaction will be aborted and Bos's balance will not change.");

    let updated_leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(updated_leases[0].1.state, LeaseState::PendingOnRent);
    println!("      ✅ Lease cannot be accepted by Bob, the state of the lease is still pending");
    Ok(())
}

#[tokio::test]
async fn test_lender_receives_a_lease_nft_after_lease_activation() -> anyhow::Result<()> {
    let context = init(NFT_NO_PAYOUT_CODE).await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test"; // leasing nft. This should match the info at nft initialisation
    let price = 10000;
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
                          "price": price.to_string(),
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id().clone()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(leases.len(), 1);
    let lease = &leases[0].1;
    assert_eq!(lease.state, LeaseState::PendingOnRent);
    println!("      ✅ Lease created");

    println!("Accepting the created lease ...");
    let lease_id = &leases[0].0;
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id().clone(),
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
    println!("      ✅ Lease acceptance confirmed");

    let active_leases: Vec<(String, LeaseCondition)> = contract
        .call("active_leases_by_lender")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(active_leases.len(), 1);
    assert_eq!(active_leases[0].1.state, LeaseState::Active);
    println!("      ✅ Lease activation confirmed");

    println!("Confirming LEASE NFT contract metatdata ...");
    let nft_contract_metadata: NFTContractMetadata = lender
        .call(contract.id(), "nft_metadata")
        .transact()
        .await?
        .json()?;

    assert_to_string_eq!(NFT_METADATA_SPEC, nft_contract_metadata.spec);
    assert_to_string_eq!(
        "NiFTyRent Lease Ownership Token",
        nft_contract_metadata.name
    );
    assert_to_string_eq!("LEASE", nft_contract_metadata.symbol);
    println!("      ✅ LEASE NFT contract metadata confirmed");

    println!("Confirming LEASE NFT enumeration ...");
    let nft_total_supply: U128 = lender
        .call(contract.id(), "nft_total_supply")
        .transact()
        .await?
        .json()?;
    assert_eq!(1, nft_total_supply.0);
    println!("      ✅ LEASE NFT total supply confirmed");

    let nft_total_supply_for_lender: U128 = lender
        .call(contract.id(), "nft_supply_for_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(1, nft_total_supply_for_lender.0);
    println!("      ✅ LEASE NFT total supply for lender confirmed");

    let lease_token_id_expected = format!("{}{}", lease_id, "_lender");
    let lease_nft_token: Option<Token> = lender
        .call(contract.id(), "nft_token")
        .args_json(json!({"token_id": lease_token_id_expected.clone()}))
        .transact()
        .await?
        .json()?;

    assert_eq!(
        lease_token_id_expected,
        lease_nft_token.as_ref().unwrap().token_id
    );
    assert_to_string_eq!(lender.id(), lease_nft_token.as_ref().unwrap().owner_id);

    let token_metadata = lease_nft_token.as_ref().unwrap().metadata.as_ref();
    assert!(token_metadata.is_some());
    assert_to_string_eq!(
        format!(
            "NiFTyRent Lease Ownership Token: {}",
            &lease_token_id_expected
        ),
        token_metadata.unwrap().title.as_ref().unwrap()
    );
    println!("      ✅ LEASE NFT nft_token info confirmed");

    let lease_nft_tokens_for_borrower: Vec<Token> = borrower
        .call(contract.id(), "nft_tokens_for_owner")
        .args_json(json!({"account_id": borrower.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(lease_nft_tokens_for_borrower.len(), 0);

    let lease_nft_tokens_for_lender: Vec<Token> = lender
        .call(contract.id(), "nft_tokens_for_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(lease_nft_tokens_for_lender.len(), 1);

    let a_lease_nft_token = &lease_nft_tokens_for_lender[0];
    assert_to_string_eq!(lender.id(), a_lease_nft_token.owner_id);
    assert_eq!(lease_token_id_expected, a_lease_nft_token.token_id);
    println!("      ✅ LEASE NFT nft_tokens_for_owner confirmed");

    let lease_nft_tokens: Vec<Token> = lender
        .call(contract.id(), "nft_tokens")
        .args_json(json!({}))
        .transact()
        .await?
        .json()?;
    assert_eq!(lease_nft_tokens.len(), 1);
    assert_eq!(lease_token_id_expected, lease_nft_tokens[0].token_id);
    println!("      ✅ LEASE NFT all nft_tokens confirmed");

    println!("      ✅ LEASE NFT token mint confirmed");
    Ok(())
}

#[tokio::test]
async fn test_lease_nft_can_be_transferred_to_other_account() -> anyhow::Result<()> {
    let context = init(NFT_NO_PAYOUT_CODE).await?;

    let lender = context.lender;
    let borrower = context.borrower;
    let lease_nft_receiver = context.lease_nft_receiver;

    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
    let worker = context.worker;
    let token_id = "test"; // leasing nft. This should match the info at nft initialisation
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
                          "price": price.to_string(),
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    println!("Accepting the created lease ...");
    let leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id().clone()}))
        .transact()
        .await?
        .json()?;
    let lease_id = &leases[0].0;
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id().clone(),
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

    // before transfer, lease is owned by the original lender
    let active_leases: Vec<(String, LeaseCondition)> = contract
        .call("active_leases_by_lender")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(active_leases.len(), 1);
    assert_eq!(active_leases[0].1.state, LeaseState::Active);
    println!("      ✅ Lease activation confirmed");

    // before transfer, lease nft is owned by the original lender
    let lease_nft_tokens_for_lender: Vec<Token> = lender
        .call(contract.id(), "nft_tokens_for_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(1, lease_nft_tokens_for_lender.len());

    let lease_token_id = format!("{}{}", lease_id, "_lender");
    let lease_nft_token: Option<Token> = lender
        .call(contract.id(), "nft_token")
        .args_json(json!({"token_id": lease_token_id.clone()}))
        .transact()
        .await?
        .json()?;
    assert_to_string_eq!(lender.id(), lease_nft_token.as_ref().unwrap().owner_id);
    println!("      ✅ LEASE NFT token got minted to the lender");

    println!("Lender Transfering the LEASE NFT to a new user ...");
    lender
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "receiver_id": lease_nft_receiver.id(),
            "token_id": lease_token_id.clone(),
        }))
        .deposit(1) //require deposit of exact 1 yocto near
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ LEASE NFT transferred");

    println!("Confirming the LEASE NFT transfer ...");
    // after transfer, the lease is owned by the new lender
    let active_leases_by_new_lender: Vec<(String, LeaseCondition)> = contract
        .call("active_leases_by_lender")
        .args_json(json!({"account_id": lease_nft_receiver.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(active_leases_by_new_lender.len(), 1);

    let active_leases_by_old_lender: Vec<(String, LeaseCondition)> = contract
        .call("active_leases_by_lender")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(active_leases_by_old_lender.len(), 0);

    // after transfer, lease nft is owned by the new lender
    let lease_nft_token: Option<Token> = lender
        .call(contract.id(), "nft_token")
        .args_json(json!({"token_id": lease_token_id.clone()}))
        .transact()
        .await?
        .json()?;
    assert_to_string_eq!(
        lease_nft_receiver.id(),
        lease_nft_token.as_ref().unwrap().owner_id
    );

    let lease_nft_tokens_for_new_lender: Vec<Token> = lender
        .call(contract.id(), "nft_tokens_for_owner")
        .args_json(json!({"account_id": lease_nft_receiver.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(1, lease_nft_tokens_for_new_lender.len());

    let lease_nft_tokens_for_old_lender: Vec<Token> = lender
        .call(contract.id(), "nft_tokens_for_owner")
        .args_json(json!({"account_id": lender.id()}))
        .transact()
        .await?
        .json()?;
    assert_eq!(0, lease_nft_tokens_for_old_lender.len());

    println!("      ✅ LEASE NFT transfer confirmed");

    Ok(())
}

#[tokio::test]
async fn test_claim_back_without_payout_using_lease_nft() -> anyhow::Result<()> {
    let context = init(NFT_NO_PAYOUT_CODE).await?;

    let lender = context.lender;
    let borrower = context.borrower;
    let lease_nft_receiver = context.lease_nft_receiver;

    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;

    let worker = context.worker;
    let token_id = "test"; // leasing nft. This should match the info at nft initialisation
    let price = 10000;
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
                          "price": price.to_string(),
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ Lease created");

    println!("Accepting the created lease ...");
    let leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id().clone()}))
        .transact()
        .await?
        .json()?;

    let lease_id = &leases[0].0;
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id().clone(),
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

    println!("Lender Transfering the LEASE NFT to a new user ...");
    let lease_token_id = format!("{}{}", lease_id, "_lender");
    lender
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "receiver_id": lease_nft_receiver.id(),
            "token_id": lease_token_id.clone(),
        }))
        .deposit(1) //require deposit of exact 1 yocto near
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ LEASE NFT transferred");

    // Fast foward to after expiration
    worker.fast_forward(12).await?;

    println!("Claiming back the NFT...");
    let balance_before_claim_back_original_lender: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;

    lease_nft_receiver
        .call(contract.id(), "claim_back")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let balance_after_claim_back_original_lender: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;

    // All fund goes to the original lender.
    assert_aprox_eq(
        balance_after_claim_back_original_lender.0 - balance_before_claim_back_original_lender.0,
        price,
    );
    println!("      ✅ Rent payout is correct");

    // NFT is sent to the lease_nft_receiver
    let tokens_for_lease_nft_receiver: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json(json!({"account_id": lease_nft_receiver.id().to_string()}))
        .transact()
        .await?
        .json()?;

    let nft_token = &tokens_for_lease_nft_receiver[0];
    assert_eq!(nft_token.token_id, token_id);
    println!("      ✅ NFT claimed back correctly");

    Ok(())
}

#[tokio::test]
async fn test_claim_back_with_payout_using_lease_nft() -> anyhow::Result<()> {
    let context = init(NFT_PAYOUT_CODE).await?;

    let lender = context.lender;
    let borrower = context.borrower;
    let lease_nft_receiver = context.lease_nft_receiver;

    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    // 2023/01/01 00:00:00
    let start_ts_nano: u64 = 1672531200000000000;
    let worker = context.worker;
    let token_id = "test"; // leasing nft. This should match the info at nft initialisation
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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
                          "price": price.to_string(),
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ Lease created");

    println!("Accepting the created lease ...");
    let leases: Vec<(String, LeaseCondition)> = contract
        .call("leases_by_owner")
        .args_json(json!({"account_id": lender.id().clone()}))
        .transact()
        .await?
        .json()?;

    let lease_id = &leases[0].0;
    borrower
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": contract.id().clone(),
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

    println!("Lender Transfering the LEASE NFT to a new user ...");
    let lease_token_id = format!("{}{}", lease_id, "_lender");
    lender
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "receiver_id": lease_nft_receiver.id(),
            "token_id": lease_token_id.clone(),
        }))
        .deposit(1) //require deposit of exact 1 yocto near
        .transact()
        .await?
        .into_result()?;
    println!("      ✅ LEASE NFT transferred");

    // Fast foward to after expiration
    worker.fast_forward(12).await?;

    println!("Claiming back the NFT...");
    let balance_before_claim_back_original_lender: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;

    let balance_before_claim_back_nft_contract: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": nft_contract.id(),
        }))
        .await?
        .json()?;

    lease_nft_receiver
        .call(contract.id(), "claim_back")
        .args_json(json!({
            "lease_id": lease_id,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let balance_after_claim_back_original_lender: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": lender.id(),
        }))
        .await?
        .json()?;

    let balance_after_claim_back_nft_contract: U128 = ft_contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": nft_contract.id(),
        }))
        .await?
        .json()?;

    // Based on the demo NFT royalty logic:
    // - the NFT contract keeps 5% of the rent.
    // - the lender receives the rest 95% of the rent.
    assert_aprox_eq(
        balance_after_claim_back_original_lender.0 - balance_before_claim_back_original_lender.0,
        price / 20 * 19,
    );
    assert_aprox_eq(
        balance_after_claim_back_nft_contract.0 - balance_before_claim_back_nft_contract.0,
        price / 20,
    );
    println!("      ✅ Rent payouts are correct");

    // NFT is sent to the lease_nft_receiver
    let tokens_for_lease_nft_receiver: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json(json!({"account_id": lease_nft_receiver.id().to_string()}))
        .transact()
        .await?
        .json()?;

    let nft_token = &tokens_for_lease_nft_receiver[0];
    assert_eq!(nft_token.token_id, token_id);
    println!("      ✅ NFT claimed back correctly");

    Ok(())
}

#[tokio::test]
async fn test_create_a_lease_to_start_in_the_future() -> anyhow::Result<()> {
    let context = init(NFT_PAYOUT_CODE).await?;
    let lender = context.lender;
    let borrower = context.borrower;
    let contract = context.rental_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let worker = context.worker;
    let token_id = "test";
    let price = 10000;
    let latest_block = worker.view_block().await?;
    let start_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 10;
    let expiration_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 100;

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
                          "start_ts_nano": start_ts_nano,
                          "end_ts_nano": expiration_ts_nano,
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

    let user_id_before_start: String = borrower
        .call(contract.id(), "get_current_user_by_contract_and_token")
        .args_json(json!({
            "contract_id": nft_contract.id(),
            "token_id": token_id,
        }))
        .transact()
        .await?
        .json()?;

    assert_eq!(lender.id().to_string(), user_id_before_start);
    println!("      ✅ The current user of this token is still the lender");

    worker.fast_forward(20).await?;
    let user_id_after_start: String = borrower
        .call(contract.id(), "get_current_user_by_contract_and_token")
        .args_json(json!({
            "contract_id": nft_contract.id(),
            "token_id": token_id,
        }))
        .transact()
        .await?
        .json()?;

    assert_eq!(borrower.id().to_string(), user_id_after_start);
    println!("      ✅ The current user of this token is borrower");

    worker.fast_forward(120).await?;
    let user_id_after_end: String = borrower
        .call(contract.id(), "get_current_user_by_contract_and_token")
        .args_json(json!({
            "contract_id": nft_contract.id(),
            "token_id": token_id,
        }))
        .transact()
        .await?
        .json()?;

    assert_eq!(lender.id().to_string(), user_id_after_end);
    println!("      ✅ The current user of this token is borrower");

    Ok(())
}

#[tokio::test]
async fn test_create_a_lease_succeeds() -> anyhow::Result<()> {
    let context = init(NFT_PAYOUT_CODE).await?;
    let worker = context.worker;
    let rental_contract = context.rental_contract;
    let marketplace_contract = context.marketplace_contract;
    let nft_contract = context.nft_contract;
    let ft_contract = context.ft_contract;
    let lender = context.lender;
    let borrower = context.borrower;
    let marketplace_owner = context.markeplace_owner;

    let token_id = "test";
    let price = 10000;
    let latest_block = worker.view_block().await?;
    let start_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 10;
    let expiration_ts_nano = latest_block.timestamp() + ONE_BLOCK_IN_NANO * 100;

    log!("Creating a listing on maketplace...");
    // lender
    //     .call(nft_contract.id(), "nft_approve")
    //     .args_json(json!({
    //         "token_id": token_id,
    //         "account_id": marketplace_contract.id(),
    //         "msg": json!({
    //             "ft_contract_addr": ft_contract.id(),
    //             "price": price.to_string(),
    //             "start_ts_nano": start_ts_nano,
    //             "end_ts_nano": expiration_ts_nano,
    //         }).to_string()
    //     }))
    //     // .deposit(1) //require deposit of exact 1 yocto near
    //     .max_gas()
    //     .transact()
    //     .await?
    //     .into_result()?;

    log!("Confirming the created listing ...");
    let result = marketplace_contract
        .call("list_listings_by_owner_id")
        .args_json(json!({"owner_id": lender.id()}))
        .transact()
        .await?;
    // log!(format!("outcome:{:?}", result.outcome()));

    assert!(result.is_success());
    let leases: Vec<(String, LeaseCondition)> = result.json()?;
    assert_eq!(leases.len(), 0);

    log!("Borrower accepting the created listing ...");

    log!("Confirm the lease is activated ...");

    Ok(())
}
// TODO: accepting a listing should activate a lease
// TODO: claim back a lease

// TODO: claim_back - NFT transfer check
// TODO: claim_back - check lease amount recieval, probably by using ft_balance_of().
// TODO: nft_on_approve - check lease createion happened correctly & all indices have been updated accordingly
// TODO: add a dummy NFT contract without payout being implemented to test the related scenarios
// TODO(syu): nft_resolve_transfer to reverse a LEASE NFT transfer
