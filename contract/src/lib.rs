use near_contract_standards::non_fungible_token::TokenId;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::bs58;
use near_sdk::collections::UnorderedMap;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, log, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault, Promise,
};

pub const TGAS: u64 = 1_000_000_000_000;
pub const XCC_GAS: Gas = Gas(5 * TGAS); // cross contract gas

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

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NftOnTransferJson {
    lease_id: String,
}

// struct for keeping track of the lease conditions
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
    lease_map: UnorderedMap<LeaseId, LeaseCondition>,
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
    pub fn lending_accept(&mut self, lease_id: LeaseId) {
        // Borrower can accept a pending lending. When this happened, the lease contract does the following:
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

        ext_nft::ext(lease_condition.contract_addr.clone())
            .with_static_gas(Gas(10 * TGAS))
            .with_attached_deposit(1)
            .nft_transfer_call(
                env::current_account_id(),                    // receiver_id
                lease_condition.token_id.clone(),             // token_id
                None,                                         // approval_id
                None,                                         // memo
                format!(r#"{{"lease_id":"{}"}}"#, &lease_id), // message should include the leaseID
            );
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
            "Queried Lease is no longer active!"
        );

        // 3. only original lender or service contract owner can claim back from expried lease
        assert!(
            (lease_condition.owner_id == env::predecessor_account_id())
                || (self.owner == env::predecessor_account_id()),
            "Only original lender or service owner can claim back!"
        );

        // 4. send rent to owner
        self.transfer(
            lease_condition.owner_id.clone(),
            lease_condition.amount_near, //TODO(syu): check if this needs to be converted to yocto
        );

        // 5. transfer nft to owner
        ext_nft::ext(lease_condition.contract_addr.clone())
            .with_static_gas(Gas(5 * TGAS))
            .with_attached_deposit(1)
            .nft_transfer(
                lease_condition.owner_id.clone(),
                lease_condition.token_id.clone(),
                None,
                None,
            );

        // 6. remove map record
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
        let promise = Promise::new(contract_id.clone());

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

// TODO: move this callback function trait to a separate file e.g. nft_callbacks.rs
/**
    Train that will handle the cross contract call from NFT contract. When nft.nft_transfer_call is called,
    it will fire a cross contract call to this_contract.nft_on_transfer(). For deails, refer to NEP-171.
*/
trait NonFungibleTokenTransferReceiver {
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId, // account that initiated the nft.nft_transfer_call(). e.g. current contract
        previous_owner_id: AccountId, // old owner of the token
        token_id: TokenId,    // NFT token id
        msg: String,
    ) -> bool;
}

#[near_bindgen]
impl NonFungibleTokenTransferReceiver for Contract {
    #[payable]
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> bool {
        // This function can only be called by initial transfer sender, which should be the current lease contract.
        assert_eq!(
            sender_id,
            env::current_account_id(),
            "sender_id does NOT match current contract id!"
        );

        let nft_on_transfer_json: NftOnTransferJson =
            near_sdk::serde_json::from_str(&msg).expect("Not valid msg for nft_on_transfer");

        log!(
            "Updating lease condition for lease_id: {}",
            &nft_on_transfer_json.lease_id
        );

        let lease_condition: LeaseCondition =
            self.lease_map.get(&nft_on_transfer_json.lease_id).unwrap();
        let new_lease_condition = LeaseCondition {
            state: LeaseState::Active,
            ..lease_condition
        };
        self.lease_map
            .insert(&nft_on_transfer_json.lease_id, &new_lease_condition);

        // all updates are completed. Return false, so that nft_resolve_transfer() from nft contract will not revert this transfer
        return false;
    }
}

// TODO: move nft callback function to separate file e.g. nft_callbacks.rs
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
    /*
    Unit test cases and helper functions
    test naming format:
    - test_{function_name}_{test_case}
    - When more than one test cases are needed for one function,
    follow the order of testing failing conditions first and success condition last
    */
    use near_sdk::env::log;
    use near_sdk::serde_json::json;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    use super::*;

    const MINT_COST: u128 = 1000000000000000000000000;

    // Helper functions
    // TODO(syu): remove input parameter and set default value
    fn get_context_builder(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let contract = Contract::new(accounts(1).into());
        assert_eq!(accounts(1), contract.owner);
        assert!(UnorderedMap::is_empty(&contract.lease_map));
    }

    #[test]
    fn test_nft_on_approve_success() {
        let mut contract = Contract::new(accounts(1).into());

        let token_id: TokenId = "test_token".to_string();
        let approval_id = 1;
        let lender: AccountId = accounts(2).into();
        let borrower: AccountId = accounts(3).into();
        let nft_address: AccountId = accounts(4).into();
        let expiration = 1000;
        let amount_near = 1;

        contract.nft_on_approve(
            token_id.clone(),
            lender.clone(),
            approval_id,
            json!({
                "contract_addr": nft_address,
                "token_id": token_id.clone(),
                "borrower": borrower,
                "expiration": expiration,
                "amount_near": amount_near.to_string()
            })
            .to_string(),
        );
        assert!(!contract.lease_map.is_empty());
        let lease_condition = &contract.leases_by_owner(lender.clone())[0].1;

        assert_eq!(nft_address, lease_condition.contract_addr);
        assert_eq!(token_id, lease_condition.token_id);
        assert_eq!(lender, lease_condition.owner_id);
        assert_eq!(borrower, lease_condition.borrower);
        assert_eq!(amount_near, lease_condition.amount_near);
        assert_eq!(expiration, lease_condition.expiration);
    }

    #[test]
    #[should_panic(expected = "Borrower is not the same one!")]
    fn test_lending_accept_wrong_borrower() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let key = "test_key".to_string();

        contract.lease_map.insert(&key, &lease_condition);

        let wrong_borrower: AccountId = accounts(4).into();
        get_context_builder(wrong_borrower.clone()).build();
        contract.lending_accept(key);
    }

    #[test]
    #[should_panic(expected = "Deposit is less than the agreed rent!")]
    fn test_lending_accept_insufficient_deposit() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        let mut builder = get_context_builder(lease_condition.borrower.clone());

        testing_env!(builder
            .attached_deposit(lease_condition.amount_near - 1)
            .build());

        contract.lending_accept(key);
    }

    #[test]
    fn test_lending_accept_success() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        let mut builder = get_context_builder(lease_condition.borrower.clone());
        testing_env!(builder
            .attached_deposit(lease_condition.amount_near)
            .build());

        contract.lending_accept(key);
    }

    #[test]
    #[should_panic(expected = "Lease has not expired yet!")]
    fn test_claim_back_not_expired_yet() {
        let mut contract = Contract::new(accounts(1).into());

        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.expiration = 1000;

        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        let mut builder = get_context_builder(lease_condition.owner_id.clone());

        testing_env!(builder
            .block_timestamp(lease_condition.expiration - 1)
            .build());
        contract.claim_back(key);
    }

    #[test]
    #[should_panic(expected = "Only original lender or service owner can claim back!")]
    fn test_claim_back_wrong_lender() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        let mut builder = get_context_builder(accounts(5).into());

        testing_env!(builder
            .block_timestamp(lease_condition.expiration + 1)
            .predecessor_account_id(accounts(5).into()) // non-owner, non-lender
            .build());

        contract.claim_back(key);
    }

    #[test]
    #[should_panic(expected = "Queried Lease is no longer active!")]
    fn test_claim_back_inactive_lease() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Expired;
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        let mut builder = get_context_builder(lease_condition.owner_id.clone());

        testing_env!(builder
            .block_timestamp(lease_condition.expiration + 1)
            .predecessor_account_id(lease_condition.owner_id.clone())
            .build());

        contract.claim_back(key);
    }

    #[test]
    #[should_panic]
    fn test_claim_back_non_exsisting_lease_id() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        let mut builder = get_context_builder(lease_condition.owner_id.clone());
        testing_env!(builder
            .block_timestamp(lease_condition.expiration + 1)
            .predecessor_account_id(lease_condition.owner_id.clone())
            .build());

        let non_existing_key = "dummy_key".to_string();
        contract.claim_back(non_existing_key);
    }

    #[test]
    fn test_claim_back_success() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.amount_near = 20;
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        let initial_balance: u128 = 100;
        let mut builder = get_context_builder(lease_condition.owner_id.clone());

        testing_env!(builder
            .storage_usage(env::storage_usage())
            .account_balance(initial_balance) //set initial balance
            .block_timestamp(lease_condition.expiration + 1)
            .build());

        contract.claim_back(key);

        testing_env!(builder
            .predecessor_account_id(lease_condition.owner_id.clone())
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .build());

        assert!(
            // service account balance should be reduced by the lease amount.
            // -1 due to gas cost
            builder.context.account_balance == (initial_balance - lease_condition.amount_near) - 1
        );
        assert!(contract.lease_map.is_empty());
        // TODO: NFT transfer check
        // TODO: ft_balance_of() to check lease amount receival.
    }

    #[test]
    fn test_leases_by_borrower() {
        todo!()
    }

    #[test]
    fn test_leases_by_owner() {
        todo!()
    }

    // Helper function to return a lease condition using default seting
    fn create_lease_condition_default() -> LeaseCondition {
        let token_id: TokenId = "test_token".to_string();
        let approval_id = 1;
        let lender: AccountId = accounts(2).into();
        let borrower: AccountId = accounts(3).into();
        let nft_address: AccountId = accounts(4).into();
        let expiration = 1000;
        let amount_near = 5;

        create_lease_condition(
            nft_address,
            token_id.clone(),
            lender.clone(),
            borrower.clone(),
            approval_id,
            expiration.clone(),
            amount_near,
            LeaseState::Pending,
        )
    }

    // Helper function create a lease condition based on input
    fn create_lease_condition(
        contract_addr: AccountId,
        token_id: TokenId,
        owner_id: AccountId,
        borrower: AccountId,
        approval_id: u64,
        expiration: u64,
        amount_near: u128,
        state: LeaseState,
    ) -> LeaseCondition {
        LeaseCondition {
            contract_addr: contract_addr,
            token_id: token_id,
            owner_id: owner_id,
            borrower: borrower,
            approval_id: approval_id,
            expiration: expiration,
            amount_near: amount_near,
            state: state,
        }
    }

    //
    // get_borrower: not found
    // get_borrower: success
}
