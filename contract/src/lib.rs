use near_contract_standards::non_fungible_token::core::NonFungibleTokenReceiver;
use near_contract_standards::non_fungible_token::TokenId;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::bs58;
use near_sdk::collections::UnorderedMap;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, log, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault, Promise,
    PromiseError,
};

pub mod externals;
pub use crate::externals::*;

type LeaseId = String;

#[derive(BorshDeserialize, BorshSerialize, Serialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
enum LeaseState {
    Pending,
    Active,
    Expired,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseJson {
    contract_addr: AccountId,
    token_id: TokenId,
    borrower: AccountId,
    expiration: u64, // TODO: duration
    amount_near: String,
}

//struct for keeping track of the lease conditions
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]

/// Details about a Lease
pub struct LeaseCondition {
    contract_addr: AccountId, // NFT contract
    token_id: TokenId,        // NFT token
    owner_id: AccountId,      // Owner of the NFT
    borrower: AccountId,      // Borrower of the NFT
    approval_id: u64,         // Approval from owner to lease
    expiration: u64,          // TODO: duration
    amount_near: u128,        // proposed lease cost
    state: LeaseState,        // current lease state
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
    lease_map: UnorderedMap<LeaseId, LeaseCondition>, // <lending_id, lending>, storing all Lease records
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    LendingsKey,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner: owner_id,
            lease_map: UnorderedMap::new(StorageKey::LendingsKey),
        }
    }

    #[payable]
    pub fn lending_accept(&mut self, lease_id: LeaseId) -> Promise {
        // A Borrower can accept the lease. When this happens, the lease contract does the following:
        // 1. Retrieve the lease data from the lease_map
        // 2. Check if the tx sender is the borrower
        // 3. Check if the deposit equals rent
        // 4. Transfer the NFT to the lease contract
        // 5. Update the lease state, when transfer succeeds

        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();
        assert!(
            lease_condition.borrower == env::predecessor_account_id(),
            "Borrower is not the same one!"
        );
        assert!(
            env::attached_deposit() >= lease_condition.amount_near,
            "Deposit is less than the agreed rent!"
        );
        // TODO: Handle the promise - Update the lease condition only when transfer succeeds
        let promise = nft::ext(lease_condition.contract_addr.clone())
            .with_static_gas(Gas(5 * TGAS))
            .with_attached_deposit(1)
            .nft_transfer(
                env::current_account_id(),
                lease_condition.token_id.clone(),
                Some(lease_condition.approval_id),
                None,
            );

        return promise.then(
            Self::ext(env::current_account_id())
                .with_static_gas(XCC_GAS)
                .lending_accept_callback(lease_id),
        );
    }

    #[private] //public - but only callable by enf::current_account_id()
    pub fn lending_accept_callback(
        &mut self,
        lease_id: LeaseId,
        #[callback_result] call_result: Result<String, PromiseError>,
    ) -> bool {
        if call_result.is_err() {
            log!("Error occured when calling nft_transfer!");
            return false;
        }

        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();
        // update the lease condition table
        let new_lease_condition = LeaseCondition {
            state: LeaseState::Active,
            ..lease_condition
        };
        self.lease_map.insert(&lease_id, &new_lease_condition);
        true
    }

    pub fn leases_by_owner(&self, account_id: AccountId) -> Vec<(String, LeaseCondition)> {
        let mut results: Vec<(String, LeaseCondition)> = vec![];
        // TODO: use better data structure to optimise this operation.
        for lease in self.lease_map.iter() {
            if lease.1.owner_id == account_id {
                results.push(lease)
            }
        }
        results
    }

    pub fn leases_by_borrower(&self, account_id: AccountId) -> Vec<(String, LeaseCondition)> {
        let mut results: Vec<(String, LeaseCondition)> = vec![];
        // TODO: use better data structure to optimise this operation.
        for lease in self.lease_map.iter() {
            if lease.1.borrower == account_id {
                results.push(lease)
            }
        }
        results
    }

    #[payable]
    pub fn claim_back(&mut self, lease_id: LeaseId) {
        // Function to allow a user to claim back the NFT and rent after a lease expired.

        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();

        // 1. check expire time
        assert!(
            lease_condition.expiration < env::block_timestamp(),
            "Lease has not expired yet!"
        );
        // 2. check state == active
        assert!(
            lease_condition.state == LeaseState::Active,
            "Querying Lease is no longer active!"
        );

        // 3. send rent to owner
        self.transfer(
            lease_condition.owner_id.clone(),
            lease_condition.amount_near,
        );

        // 4. transfer nft to owner
        let promise = nft::ext(lease_condition.contract_addr.clone())
            .with_static_gas(Gas(5 * TGAS))
            .with_attached_deposit(1)
            .nft_transfer(
                lease_condition.owner_id.clone(),
                lease_condition.token_id.clone(),
                None,
                None,
            );

        // 5. remove map record
        self.lease_map.remove(&lease_id);
    }

    fn transfer(&self, to: AccountId, amount: Balance) {
        // helper function to perform FT transfer
        Promise::new(to).transfer(amount);
    }

    pub fn get_borrower(&self, contract_id: AccountId, token_id: TokenId) -> Option<AccountId> {
        // return the current borrower of the NFTs
        // TODO: use better data structure to optimise this operation.
        for lease in self.lease_map.iter() {
            if (lease.1.contract_addr == contract_id) && (lease.1.token_id == token_id) {
                return Some(lease.1.borrower);
            }
        }
        return None;
    }

    pub fn proxy_func_calls(&self, contract_id: AccountId, method_name: String, args: String) {
        // proxy function to open accessible functions calls in a NFT contract during lease
        let mut promise = Promise::new(contract_id.clone());

        // TODO: allow the lend to define white list of method names.
        // unreachable methods in leased NFT contract
        assert_ne!(
            "nft_transfer", &method_name,
            "Calling method is not accessiable!"
        );
        assert_ne!(
            "nft_approve", &method_name,
            "Calling method is not accessiable!"
        );

        promise.function_call(
            method_name.clone(),
            args.into(),
            env::attached_deposit(),
            Gas(5 * TGAS),
        );
    }
}

/**
    trait that will be used as the callback from the NFT contract. When nft_approve is
    called, it will fire a cross contract call to this marketplace and this is the function
    that is invoked.
*/
trait NonFungibleTokenApprovalsReceiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    );
}
#[near_bindgen]
impl NonFungibleTokenApprovalsReceiver for Contract {
    /// where we add the sale because we know nft owner can only call nft_approve
    #[payable]
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    ) {
        //the lease conditions come from the msg field
        let lease_json: LeaseJson =
            near_sdk::serde_json::from_str(&msg).expect("Not valid lease data");

        assert_eq!(token_id, lease_json.token_id);

        // build lease condition from the parsed json
        let lease_condition: LeaseCondition = LeaseCondition {
            owner_id: owner_id.clone(),
            approval_id,
            contract_addr: lease_json.contract_addr,
            token_id: lease_json.token_id,
            borrower: lease_json.borrower,
            expiration: lease_json.expiration,
            amount_near: lease_json.amount_near.parse::<u128>().unwrap(),
            state: LeaseState::Pending,
        };

        let seed = near_sdk::env::random_seed();
        let key = bs58::encode(seed)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string();
        self.lease_map.insert(&key, &lease_condition);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    use super::*;

    const MINT_COST: u128 = 1000000000000000000000000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    // TODO: Add tests
    #[test]
    fn test_new() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_COST)
            .predecessor_account_id(accounts(0))
            .build());
    }
}
