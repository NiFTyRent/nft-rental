use near_sdk::serde::{Deserialize, Serialize};
use near_units::parse_near;
use serde_json::json;
use workspaces::{network::Sandbox, Account, Contract, Worker};

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
        .args_json(json!({ "owner_id": alice.id() }))
        .transact()
        .await?
        .into_result()?;

    Ok(Context {
        lender: alice,
        borrower: bob,
        contract,
        nft_contract,
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

async fn prepare_lease(context: &Context, token_id: String) -> anyhow::Result<()> {
    let lender = &context.lender;
    let borrower = &context.borrower;
    let contract = &context.contract;
    let nft_contract = &context.nft_contract;

    let expiration_ts_nano = 1000;
    lender
        .call( nft_contract.id(), "nft_mint")
        .args_json(
            json!({ "token_id": token_id, "receiver_id": lender.id(), "token_metadata": {"title": "Test"}}),
        )
        .deposit(parse_near!("0.1 N"))
        .transact()
        .await?.into_result()?;
    lender
        .call(nft_contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": token_id,
            "account_id": contract.id(),
            "msg": json!({"contract_addr": nft_contract.id(),
                          "token_id": token_id,
                          "borrower": borrower.id(),
                          "expiration": expiration_ts_nano,
                          "amount_near": "1"
            }).to_string()
        }))
        .deposit(parse_near!("1 N"))
        .transact()
        .await?
        .into_result()?;

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
    Ok(())
}

#[tokio::test]
async fn get_borrower() -> anyhow::Result<()> {
    let context = init().await?;
    let borrower = context.borrower.clone();
    let contract = context.contract.clone();
    let nft_contract = context.nft_contract.clone();

    println!("Prepare 20 leases ...");
    tokio::join!(
        prepare_lease(&context, "0".to_string()),
        prepare_lease(&context, "1".to_string()),
        prepare_lease(&context, "2".to_string()),
        prepare_lease(&context, "3".to_string()),
        prepare_lease(&context, "4".to_string()),
        prepare_lease(&context, "5".to_string()),
        prepare_lease(&context, "6".to_string()),
        prepare_lease(&context, "7".to_string()),
        prepare_lease(&context, "8".to_string()),
        prepare_lease(&context, "9".to_string()),
        prepare_lease(&context, "10".to_string()),
        prepare_lease(&context, "11".to_string()),
        prepare_lease(&context, "12".to_string()),
        prepare_lease(&context, "13".to_string()),
        prepare_lease(&context, "14".to_string()),
        prepare_lease(&context, "15".to_string()),
        prepare_lease(&context, "16".to_string()),
        prepare_lease(&context, "17".to_string()),
        prepare_lease(&context, "18".to_string()),
        prepare_lease(&context, "19".to_string()),
    );

    println!("Querying the borrower");
    let res = borrower
        .call(contract.id(), "get_borrower")
        .args_json(json!({
            "contract_id": nft_contract.id(),
            "token_id": "10",
        }))
        .transact()
        .await?;

    println!(
        "    ⛽Total burnt gas: {} TGas",
        res.total_gas_burnt as f64 / 1e12
    );

    Ok(())
}
